use std::collections::{HashMap, VecDeque};

use radius_sdk::{
    kvstore::{KvStoreError, Model},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_generation::{PartialKey as SkdePartialKey, PartialKeyProof};

use crate::types::{KeyId, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionPartialKey {
    pub address: Address,
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub timestamp: u64,
    pub index: Option<usize>,
    pub signature: Signature,
    pub ack_timestamp: Option<u64>,
    pub ack_signature: Option<Signature>,
}

// 세션 내 복호화 키 데이터
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionDecryptionKey {
    pub decryption_key: String,
    pub solver_address: Address,
    pub timestamp: u64,
    pub signature: Signature,
    pub ack_timestamp: Option<u64>,
    pub ack_signature: Option<Signature>,
}

// 세션 데이터
#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: &SessionId))]
pub struct SessionData {
    pub key_id: KeyId,
    pub partial_keys: HashMap<Address, SessionPartialKey>,
    pub decryption_key: Option<SessionDecryptionKey>,
    pub final_reveal_sent: bool,
}

impl SessionData {
    pub fn new(key_id: KeyId) -> Self {
        Self {
            key_id,
            partial_keys: HashMap::new(),
            decryption_key: None,
            final_reveal_sent: false,
        }
    }

    pub fn add_partial_key(
        &mut self,
        address: Address,
        partial_key: SkdePartialKey,
        proof: PartialKeyProof,
        timestamp: u64,
        signature: Signature,
    ) {
        let session_partial_key = SessionPartialKey {
            address: address.clone(),
            partial_key,
            proof,
            timestamp,
            index: None,
            signature,
            ack_timestamp: None,
            ack_signature: None,
        };

        self.partial_keys.insert(address, session_partial_key);
    }

    pub fn ack_partial_key(
        &mut self,
        address: &Address,
        index: usize,
        ack_timestamp: u64,
        ack_signature: Signature,
    ) -> bool {
        if let Some(partial_key) = self.partial_keys.get_mut(address) {
            partial_key.index = Some(index);
            partial_key.ack_timestamp = Some(ack_timestamp);
            partial_key.ack_signature = Some(ack_signature);
            true
        } else {
            false
        }
    }

    pub fn set_decryption_key(
        &mut self,
        decryption_key: String,
        solver_address: Address,
        timestamp: u64,
        signature: Signature,
    ) {
        self.decryption_key = Some(SessionDecryptionKey {
            decryption_key,
            solver_address,
            timestamp,
            signature,
            ack_timestamp: None,
            ack_signature: None,
        });
    }

    pub fn ack_decryption_key(&mut self, ack_timestamp: u64, ack_signature: Signature) -> bool {
        if let Some(decryption_key) = self.decryption_key.as_mut() {
            decryption_key.ack_timestamp = Some(ack_timestamp);
            decryption_key.ack_signature = Some(ack_signature);
            true
        } else {
            false
        }
    }

    pub fn mark_final_reveal_sent(&mut self) {
        self.final_reveal_sent = true;
    }
}

// 세션 관리자: 최근 세션만 메모리에 유지
#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key())]
pub struct SessionManager {
    recent_sessions: VecDeque<SessionId>,
    max_sessions: usize,
}

impl SessionManager {
    pub fn new(max_sessions: usize) -> Self {
        Self {
            recent_sessions: VecDeque::with_capacity(max_sessions),
            max_sessions,
        }
    }

    pub fn default() -> Self {
        Self::new(10) // 기본값으로 최대 10개 세션 유지
    }

    pub fn add_session(&mut self, session_id: SessionId) {
        // 이미 있는 세션은 추가하지 않음
        if self.recent_sessions.contains(&session_id) {
            return;
        }

        // 최대 세션 수를 초과하면 가장 오래된 세션 제거
        if self.recent_sessions.len() >= self.max_sessions {
            if let Some(oldest_session_id) = self.recent_sessions.pop_front() {
                // KV 저장소에서도 제거 (에러는 무시)
                let _ = SessionData::delete(&oldest_session_id);
            }
        }

        self.recent_sessions.push_back(session_id);
    }

    pub fn get_recent_sessions(&self) -> Vec<SessionId> {
        self.recent_sessions.iter().cloned().collect()
    }

    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let session_manager = Self::default();
            session_manager.put()?;
        }

        Ok(())
    }
}
