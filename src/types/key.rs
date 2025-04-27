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
