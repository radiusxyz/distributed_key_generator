use std::sync::Arc;

use once_cell::sync::OnceCell;
use radius_sdk::{kvstore::KvStoreError, signature::Address};
use skde::{delay_encryption::SkdeParams, key_generation::PartialKeyProof};
use tracing::info;

use crate::{
    state::AppState,
    types::{KeyIdCounter, PartialKey, PartialKeyPool, SessionId, UsedPartialKeysList},
    utils::generate_partial_key_with_proof,
};

const MAX_KEYS: usize = 10;
const MIN_THRESHOLD: usize = 5;

// Global singleton instance
static PARTIAL_KEY_MANAGER: OnceCell<Arc<PartialKeyManager>> = OnceCell::new();

pub struct PartialKeyManager {
    max_keys: usize,
    min_threshold: usize,
}

impl PartialKeyManager {
    pub fn new(max_keys: usize, min_threshold: usize) -> Self {
        Self {
            max_keys,
            min_threshold,
        }
    }

    /// Get the global singleton instance of the PartialKeyManager
    pub fn global() -> Arc<PartialKeyManager> {
        PARTIAL_KEY_MANAGER
            .get_or_init(|| Arc::new(PartialKeyManager::new(MAX_KEYS, MIN_THRESHOLD)))
            .clone()
    }

    /// Initialize the PartialKeyManager
    pub async fn initialize() -> Result<(), KvStoreError> {
        // Initialize key ID counter
        KeyIdCounter::initialize()?;
        Ok(())
    }

    /// Fill the pool of partial keys up to the maximum limit
    pub async fn add_keys_to_pool(&self, skde_params: &SkdeParams) -> Result<(), KvStoreError> {
        let available_keys = self.available_key_count().await?;

        if available_keys >= self.max_keys {
            return Ok(());
        }

        for _ in available_keys..self.max_keys {
            // Allocate new key ID
            let key_id = KeyIdCounter::get_next_id_and_increment()?;

            let (partial_key, proof) = generate_partial_key_with_proof(skde_params);
            let precomputed_key = PartialKeyPool::new(key_id, partial_key, proof);

            precomputed_key.put(key_id)?;
        }

        let available_keys = self.available_key_count().await?;
        info!("PartialKeyManager: Added {} keys to pool", available_keys);

        Ok(())
    }

    /// Get a fresh partial key for a specific session
    pub async fn get_fresh_partial_key_for_session(
        &self,
        session_id: SessionId,
        address: &Address,
    ) -> Result<Option<(PartialKey, PartialKeyProof)>, KvStoreError> {
        // Find an available key
        match PartialKeyPool::find_first_available()? {
            Some(mut precomputed_key) => {
                let key_id = precomputed_key.id;

                // Mark key as used
                precomputed_key.mark_as_used(session_id);
                precomputed_key.put(key_id)?;

                // Decrement available key count
                KeyIdCounter::decrement_available_count()?;

                // Initialize used key list for the session with the used key ID
                UsedPartialKeysList::initialize(session_id, key_id)?;

                // Create and store PartialKey
                let partial_key = PartialKey::new(precomputed_key.partial_key().clone());
                let proof = precomputed_key.proof().clone();
                partial_key.put(session_id, address)?;

                Ok(Some((partial_key, proof)))
            }
            None => {
                // No available key
                Ok(None)
            }
        }
    }

    /// Get the used key ID for a specific session
    pub async fn get_used_key_id_for_session(
        &self,
        session_id: SessionId,
    ) -> Result<Option<u64>, KvStoreError> {
        match UsedPartialKeysList::get(session_id) {
            Ok(used_keys_list) => Ok(Some(used_keys_list.used_key_id)),
            Err(_) => Ok(None),
        }
    }

    /// Get the partial key used in a specific session
    pub async fn get_used_key_for_session(
        &self,
        session_id: SessionId,
        my_address: &Address,
    ) -> Result<Option<PartialKey>, KvStoreError> {
        // 로직 변경: 현재 노드 주소에서만 키를 찾음
        match PartialKey::get(session_id, my_address) {
            Ok(key) => Ok(Some(key)),
            Err(_) => Ok(None),
        }
    }

    /// Get the count of available keys
    pub async fn available_key_count(&self) -> Result<usize, KvStoreError> {
        KeyIdCounter::get_available_count()
    }

    /// Check if a key has been used in a specific session
    pub async fn has_used_key(&self, session_id: SessionId) -> Result<bool, KvStoreError> {
        match UsedPartialKeysList::get(session_id) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// Run the partial key manager service
pub async fn run_partial_key_manager(context: AppState) {
    // Create singleton instance
    let manager = PartialKeyManager::global();

    // Initialize
    if let Err(e) = PartialKeyManager::initialize().await {
        tracing::error!("Failed to initialize PartialKeyManager: {}", e);
        return;
    }

    info!("PartialKeyManager initialized");

    loop {
        match manager.available_key_count().await {
            Ok(available) => {
                if available < manager.min_threshold {
                    match manager.add_keys_to_pool(context.skde_params()).await {
                        Ok(_) => {
                            info!(
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
