use std::collections::HashSet;

use radius_sdk::{
    kvstore::{KvStoreError, Model},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_aggregation::AggregatedKey as SkdeAggregatedKey;

use crate::rpc::{cluster::SubmitPartialKey, common::PartialKeyPayload};

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId, address: &Address))]
pub struct PartialKeySubmission {
    pub signature: Signature,
    pub payload: PartialKeyPayload,
}

impl PartialKeySubmission {
    pub fn new(partial_key_submission: &PartialKeySubmission) -> Self {
        Self {
            signature: partial_key_submission.signature.clone(),
            payload: partial_key_submission.payload.clone(),
        }
    }

    pub fn from_submit_partial_key(submit_partial_key: &SubmitPartialKey) -> Self {
        Self {
            signature: submit_partial_key.signature.clone(),
            payload: submit_partial_key.payload.clone(),
        }
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
            let partial_key_address_list = PartialKeyAddressList::default();

            partial_key_address_list.put(session_id)?;
        }

        Ok(())
    }

    pub fn get_partial_key_list(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<PartialKeySubmission>, KvStoreError> {
        let partial_key_submissions: Result<Vec<PartialKeySubmission>, _> = self
            .0
            .iter()
            .map(|address| PartialKeySubmission::get(session_id, address))
            .collect();

        partial_key_submissions?
            .into_iter()
            .map(|partial_key_submission| Ok(partial_key_submission))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Model, Default, Hash)]
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
