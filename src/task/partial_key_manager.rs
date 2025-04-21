// Draft code.
use std::sync::Arc;

use once_cell::sync::OnceCell;
use radius_sdk::{
    kvstore::{KvStoreError, Model},
    signature::Address,
};
use skde::{
    delay_encryption::SkdeParams,
    key_generation::{
        generate_partial_key, prove_partial_key_validity, PartialKey as SkdePartialKey,
        PartialKeyProof,
    },
};
use tracing;

use crate::{
    state::AppState,
    types::{PartialKey, SessionId},
    utils::get_current_timestamp,
};

// 싱글톤 인스턴스를 위한 전역 변수
static PARTIAL_KEY_MANAGER: OnceCell<Arc<PartialKeyManager>> = OnceCell::new();

// 키 ID를 관리하는 카운터 (다음 사용 가능한 ID 추적)
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Model)]
#[kvstore(key())]
pub struct KeyIdCounter {
    next_id: u64,
    available_count: usize,
}

impl KeyIdCounter {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            available_count: 0,
        }
    }

    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let counter = Self::new();
            counter.put()?;
        }
        Ok(())
    }

    // 다음 ID 가져오기와 함께 카운트 증가
    pub fn get_next_id_and_increment() -> Result<u64, KvStoreError> {
        let mut result = 0;
        KeyIdCounter::apply(|counter| {
            result = counter.next_id;
            counter.next_id += 1;
            counter.available_count += 1;
        })?;
        Ok(result)
    }

    // 사용 가능한 키 개수 감소
    pub fn decrement_available_count() -> Result<(), KvStoreError> {
        Self::apply(|counter| {
            if counter.available_count > 0 {
                counter.available_count -= 1;
            }
        })
    }

    // 사용 가능한 키 개수 조회
    pub fn get_available_count() -> Result<usize, KvStoreError> {
        match Self::get() {
            Ok(counter) => Ok(counter.available_count),
            Err(e) => {
                if let KvStoreError::Get(_) = e {
                    Ok(0) // 카운터가 없으면 0개로 간주
                } else {
                    Err(e)
                }
            }
        }
    }
}

pub struct PartialKeyManager {
    max_keys: usize,
    min_threshold: usize,
}

// PrecomputedPartialKey는 시스템에서 미리 생성한 partial key를 관리하기 위한 모델
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Model)]
#[kvstore(key(id: u64))]
pub struct PrecomputedPartialKey {
    id: u64,
    used_in_session: Option<SessionId>, // 사용된 세션 ID, None이면 사용 가능
    partial_key: SkdePartialKey,
    proof: PartialKeyProof,
    timestamp: u64,
}

impl PrecomputedPartialKey {
    pub fn new(id: u64, partial_key: SkdePartialKey, proof: PartialKeyProof) -> Self {
        Self {
            id,
            used_in_session: None,
            partial_key,
            proof,
            timestamp: get_current_timestamp(),
        }
    }

    pub fn partial_key(&self) -> &SkdePartialKey {
        &self.partial_key
    }

    pub fn proof(&self) -> &PartialKeyProof {
        &self.proof
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn is_available(&self) -> bool {
        self.used_in_session.is_none()
    }

    pub fn find_first_available() -> Result<Option<Self>, KvStoreError> {
        KeyIdCounter::initialize()?;

        let counter = KeyIdCounter::get()?;
        let max_id = counter.next_id;

        for id in 0..max_id {
            match Self::get(id) {
                Ok(key) => {
                    if key.is_available() {
                        return Ok(Some(key));
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(None)
    }

    // 키를 특정 세션에 사용한 것으로 표시
    pub fn mark_as_used(&mut self, session_id: SessionId) {
        self.used_in_session = Some(session_id);
    }
}

// UsedPartialKey는 특정 세션에서 사용된 partial key를 관리하기 위한 모델
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct UsedPartialKeysList {
    used_key_addresses: Vec<Address>,
    used_key_ids: Vec<u64>, // 사용된 키 ID 목록 추가
}

impl UsedPartialKeysList {
    pub fn new() -> Self {
        Self {
            used_key_addresses: Vec::new(),
            used_key_ids: Vec::new(),
        }
    }

    pub fn add_key_address(&mut self, address: Address) {
        self.used_key_addresses.push(address);
    }

    pub fn add_key_id(&mut self, key_id: u64) {
        self.used_key_ids.push(key_id);
    }

    pub fn contains(&self, address: &Address) -> bool {
        self.used_key_addresses.contains(address)
    }

    pub fn initialize(session_id: SessionId) -> Result<(), KvStoreError> {
        if Self::get(session_id).is_err() {
            let used_keys = UsedPartialKeysList::new();
            used_keys.put(session_id)?;
        }
        Ok(())
    }
}

impl PartialKeyManager {
    pub fn new(max_keys: usize, min_threshold: usize) -> Self {
        Self {
            max_keys,
            min_threshold,
        }
    }

    // 싱글톤 인스턴스 getter
    pub fn global() -> Arc<PartialKeyManager> {
        PARTIAL_KEY_MANAGER
            .get_or_init(|| Arc::new(PartialKeyManager::new(5, 2)))
            .clone()
    }

    pub async fn initialize() -> Result<(), KvStoreError> {
        // 키 ID 카운터 초기화
        KeyIdCounter::initialize()?;
        Ok(())
    }

    pub async fn generate_keys_if_needed(
        &self,
        skde_params: &SkdeParams,
    ) -> Result<(), KvStoreError> {
        let available_keys = self.available_key_count().await?;

        if available_keys >= self.max_keys {
            return Ok(());
        }

        for _ in available_keys..self.max_keys {
            // 새 키 ID 할당
            let key_id = KeyIdCounter::get_next_id_and_increment()?;
            tracing::info!("Generating key: {}", key_id);

            let (partial_key, proof) = generate_partial_key_somehow(skde_params);
            let precomputed_key = PrecomputedPartialKey::new(key_id, partial_key, proof);

            precomputed_key.put(key_id)?;
        }

        Ok(())
    }

    // 세션 ID와 함께 PartialKey를 가져오는 함수
    pub async fn get_fresh_partial_key_for_session(
        &self,
        session_id: SessionId,
        address: &Address,
    ) -> Result<Option<(PartialKey, PartialKeyProof)>, KvStoreError> {
        // 사용 가능한 키 찾기
        match PrecomputedPartialKey::find_first_available()? {
            Some(mut precomputed_key) => {
                let key_id = precomputed_key.id;

                // 키를 사용한 것으로 표시
                precomputed_key.mark_as_used(session_id);
                precomputed_key.put(key_id)?;

                // 가용 키 카운트 감소
                KeyIdCounter::decrement_available_count()?;

                // 세션에 대한 사용된 키 목록 초기화
                UsedPartialKeysList::initialize(session_id)?;

                // 세션에 사용된 키로 추가
                UsedPartialKeysList::apply(session_id, |list| {
                    list.add_key_address(address.clone());
                    list.add_key_id(key_id);
                })?;

                // PartialKey 생성 및 저장
                let partial_key = PartialKey::new(precomputed_key.partial_key().clone());
                let proof = precomputed_key.proof().clone();
                partial_key.put(session_id, address)?;

                Ok(Some((partial_key, proof)))
            }
            None => {
                // 사용 가능한 키가 없음
                Ok(None)
            }
        }
    }

    // 특정 세션에서 사용한 PartialKey 리스트 조회
    pub async fn get_used_keys_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<PartialKey>, KvStoreError> {
        if let Ok(used_keys_list) = UsedPartialKeysList::get(session_id) {
            let mut result = Vec::new();

            for address in &used_keys_list.used_key_addresses {
                if let Ok(key) = PartialKey::get(session_id, address) {
                    result.push(key);
                }
            }

            Ok(result)
        } else {
            Ok(Vec::new())
        }
    }

    // 특정 세션에서 사용된 키 ID 목록 조회
    pub async fn get_used_key_ids_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<u64>, KvStoreError> {
        if let Ok(used_keys_list) = UsedPartialKeysList::get(session_id) {
            Ok(used_keys_list.used_key_ids.clone())
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn available_key_count(&self) -> Result<usize, KvStoreError> {
        KeyIdCounter::get_available_count()
    }

    pub async fn used_key_count(&self, session_id: SessionId) -> Result<usize, KvStoreError> {
        if let Ok(used_keys_list) = UsedPartialKeysList::get(session_id) {
            Ok(used_keys_list.used_key_addresses.len())
        } else {
            Ok(0)
        }
    }
}

// 새로운 PartialKey와 proof를 생성하는 함수
fn generate_partial_key_somehow(
    skde_params: &SkdeParams,
) -> (SkdePartialKey, skde::key_generation::PartialKeyProof) {
    // skde 라이브러리를 이용해 실제 PartialKey 생성
    let (secret_value, skde_partial_key) = generate_partial_key(skde_params).unwrap();

    // 키에 대한 증명 생성
    let proof = prove_partial_key_validity(skde_params, &secret_value).unwrap();

    (skde_partial_key, proof)
}

pub async fn run_partial_key_manager(context: AppState) {
    // 싱글톤으로 생성
    let manager = PartialKeyManager::global();

    // 초기화 - 정적 메서드로 호출
    if let Err(e) = PartialKeyManager::initialize().await {
        tracing::error!("Failed to initialize PartialKeyManager: {}", e);
        return;
    }

    print!("PartialKeyManager initialized");

    loop {
        match manager.available_key_count().await {
            Ok(available) => {
                if available < manager.min_threshold {
                    match manager.generate_keys_if_needed(context.skde_params()).await {
                        Ok(_) => {
                            tracing::info!(
                                "Partial Key Manager: Generated partial keys. Available: {}",
                                available
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Partial Key Manager: Failed to generate partial keys: {}",
                                e
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to get available key count: {}", e);
            }
        }
    }
}
