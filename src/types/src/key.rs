use std::collections::HashSet;

use radius_sdk::{
    kvstore::{KvStoreError, Model},
    signature::{Address, Signature},
};
use serde::{Deserialize, Serialize};
use skde::key_aggregation::AggregatedKey as SkdeAggregatedKey;
use crate::{rpc::{cluster::SubmitPartialKey, common::PartialKeyPayload}, Error};

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
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct PartialKeyAddressList(HashSet<Address>);

impl PartialKeyAddressList {
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
        Self(HashSet::new()).put(session_id)
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
        Self(value)
    }
}

impl Into<u64> for SessionId {
    fn into(self) -> u64 {
        self.0
    }
}

impl SessionId {

    pub fn initialize() -> Result<(), KvStoreError> {
        Self(0).put()
    }

    pub fn is_initial(&self) -> bool {
        self.0 == 0
    }

    pub fn prev(self) -> Option<Self> {
       self.0.checked_sub(1).map(Self)
    }

    pub fn next(&self) -> Option<Self> {
        self.0.checked_add(1).map(Self)
    }

    pub fn next_mut(&mut self) -> Result<(), Error> {
        self.0 = self.next().ok_or(Error::Arithmetic)?.into();
        Ok(())
    }

    pub fn prev_mut(&mut self) -> Result<(), Error> {
        self.0 = self.prev().ok_or(Error::Arithmetic)?.into();
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct DecryptionKey(String);

impl Into<String> for DecryptionKey {
    fn into(self) -> String {
        self.0
    }
}

impl DecryptionKey {
    pub fn new(decryption_key: String) -> Self {
        Self(decryption_key)
    }

    pub fn to_bytes(self) -> Vec<u8> {
        self.0.into_bytes()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct AggregatedKey(SkdeAggregatedKey);

impl AggregatedKey {
    pub fn new(aggregated_key: SkdeAggregatedKey) -> Self {
        Self(aggregated_key)
    }

    pub fn enc_key(self) -> String {
        self.0.u
    }
}
