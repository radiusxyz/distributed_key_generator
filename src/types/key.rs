use std::collections::HashSet;

use radius_sdk::{
    kvstore::{KvStoreError, Model},
    signature::Address,
};
use serde::{Deserialize, Serialize};
use skde::{
    key_aggregation::AggregatedKey as SkdeAggregatedKey,
    key_generation::{PartialKey as SkdePartialKey, PartialKeyProof},
};

use crate::utils::get_current_timestamp;

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId, address: &Address))]
pub struct PartialKey(SkdePartialKey);

impl PartialKey {
    pub fn new(partial_key: SkdePartialKey) -> Self {
        Self(partial_key)
    }

    pub fn into_inner(self) -> SkdePartialKey {
        self.0
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct PartialKeyAddressList(HashSet<Address>);

impl PartialKeyAddressList {
    pub fn default() -> Self {
        Self(HashSet::new())
    }

    pub fn insert(&mut self, address: Address) {
        self.0.insert(address);
    }

    pub fn remove(&mut self, address: Address) {
        self.0.remove(&address);
    }

    pub fn to_vec(&self) -> Vec<Address> {
        self.0.iter().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn initialize(session_id: SessionId) -> Result<(), KvStoreError> {
        if Self::get(session_id).is_err() {
            let partial_key_list = PartialKeyAddressList::default();

            partial_key_list.put(session_id)?;
        }

        Ok(())
    }

    pub fn get_partial_key_list(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<SkdePartialKey>, KvStoreError> {
        let partial_key_list: Result<Vec<PartialKey>, _> = self
            .0
            .iter()
            .map(|address| PartialKey::get(session_id, address))
            .collect();

        partial_key_list?
            .into_iter()
            .map(|partial_key| Ok(partial_key.into_inner()))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Model, Default)]
#[kvstore(key())]
pub struct SessionId(u64);

impl From<u64> for SessionId {
    fn from(value: u64) -> Self {
        SessionId(value)
    }
}

impl SessionId {
    pub fn default() -> Self {
        Self(0)
    }

    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let session_id = Self::default();

            session_id.put()?
        }

        Ok(())
    }

    pub fn increase_session_id(&mut self) {
        self.0 += 1;
    }

    pub fn decrease_session_id(&mut self) {
        self.0 -= 1;
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct DecryptionKey(String);

impl DecryptionKey {
    pub fn new(decryption_key: String) -> Self {
        Self(decryption_key)
    }

    pub fn as_string(self) -> String {
        self.0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct AggregatedKey(SkdeAggregatedKey);

impl AggregatedKey {
    pub fn new(aggregated_key: SkdeAggregatedKey) -> Self {
        Self(aggregated_key)
    }

    pub fn encryption_key(self) -> String {
        self.0.u
    }
}

/// A counter that tracks the next available key ID and number of available keys
#[derive(Clone, Debug, Deserialize, Serialize, Model)]
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

    /// Initialize the key ID counter in storage if it doesn't exist
    pub fn initialize() -> Result<(), KvStoreError> {
        if Self::get().is_err() {
            let counter = Self::new();
            counter.put()?;
        }
        Ok(())
    }

    /// Get the next available ID and increment the counter
    pub fn get_next_id_and_increment() -> Result<u64, KvStoreError> {
        let mut result = 0;
        KeyIdCounter::apply(|counter| {
            result = counter.next_id;
            counter.next_id += 1;
            counter.available_count += 1;
        })?;
        Ok(result)
    }

    /// Decrease the count of available keys
    pub fn decrement_available_count() -> Result<(), KvStoreError> {
        Self::apply(|counter| {
            if counter.available_count > 0 {
                counter.available_count -= 1;
            }
        })
    }

    /// Get the count of available keys
    pub fn get_available_count() -> Result<usize, KvStoreError> {
        match Self::get() {
            Ok(counter) => Ok(counter.available_count),
            Err(e) => {
                if let KvStoreError::Get(_) = e {
                    Ok(0) // If counter doesn't exist, assume 0 available keys
                } else {
                    Err(e)
                }
            }
        }
    }
}

/// A precomputed partial key stored in the system
#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(id: u64))]
pub struct PrecomputedPartialKey {
    pub id: u64,
    pub used_in_session: Option<SessionId>, // None if available, Some(session_id) if used
    pub partial_key: SkdePartialKey,
    pub proof: PartialKeyProof,
    pub timestamp: u64,
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

    /// Find the first available precomputed key
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

    /// Mark this key as used in a specific session
    pub fn mark_as_used(&mut self, session_id: SessionId) {
        self.used_in_session = Some(session_id);
    }
}

/// Tracks which key ID has been used in a specific session
#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct UsedPartialKeysList {
    pub used_key_id: u64,
}

impl UsedPartialKeysList {
    pub fn new(key_id: u64) -> Self {
        Self {
            used_key_id: key_id,
        }
    }

    /// Initialize the used key list for a session with the specific key ID
    pub fn initialize(session_id: SessionId, key_id: u64) -> Result<(), KvStoreError> {
        if Self::get(session_id).is_err() {
            let used_keys = UsedPartialKeysList::new(key_id);
            used_keys.put(session_id)?;
        }
        Ok(())
    }
}
