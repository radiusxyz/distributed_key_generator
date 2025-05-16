use std::collections::HashSet;
use std::hash::Hash;
use radius_sdk::kvstore::{KvStoreError, Model};
use serde::{Deserialize, Serialize};
use skde::key_aggregation::AggregatedKey as SkdeAggregatedKey;

use crate::{AppState, PartialKeyPayload, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId, address: &Address))]
pub struct PartialKeySubmission<Signature, Address> {
    pub signature: Signature,
    pub payload: PartialKeyPayload<Address>,
}

impl<Signature, Address> PartialKeySubmission<Signature, Address> {
    pub fn new(signature: Signature, payload: PartialKeyPayload<Address>) -> Self {
        Self { signature, payload }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
pub struct PartialKeyAddressList<Address: Hash + Eq + Clone>(HashSet<Address>);

impl<Address> PartialKeyAddressList<Address> 
where
    Address: Hash + Eq + Clone + Serialize,
{
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

    pub fn get_partial_key_list<C>(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<PartialKeySubmission<C::Signature, C::Address>>, KvStoreError> 
    where
        C: AppState,
    {
        let partial_key_submissions: Result<Vec<PartialKeySubmission<C::Signature, C::Address>>, _> = self
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
