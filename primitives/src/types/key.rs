use crate::{AppState, PartialKeyPayload, SessionId};
use std::fmt::{Debug, Display};
use radius_sdk::kvstore::{KvStoreError, Model};
use crate::traits::{AddressT, Parameter};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use skde::key_aggregation::AggregatedKey as SkdeAggregatedKey;
use skde::key_generation::PartialKey;

#[derive(Clone, Debug, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId, address: Address))]
pub struct PartialKeySubmission<Signature, Address> {
    pub signature: Signature,
    pub payload: PartialKeyPayload<Address>,
}

impl<Signature, Address> PartialKeySubmission<Signature, Address> {
    pub fn new(signature: Signature, payload: PartialKeyPayload<Address>) -> Self {
        Self { signature, payload }
    }

    pub fn sender(&self) -> &Address {
        &self.payload.sender
    }

    pub fn partial_key(&self) -> PartialKey {
        self.payload.partial_key.clone()
    }
}

impl<Signature, Address: Debug> Display for PartialKeySubmission<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Broadcasting partial key acknowledgment - PartialKeySubmission {{ sender: {:?}, session_id: {:?}, timestamp: {} }}", 
            self.payload.sender, 
            self.payload.session_id, 
            self.payload.submit_timestamp
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Model)]
#[kvstore(key(session_id: SessionId))]
/// List of partial key submitters
pub struct SubmitterList<Address>(Vec<Address>);

impl<Address: Parameter + AddressT> SubmitterList<Address> {

    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, address: Address) {
        self.0.push(address);
    }

    pub fn remove(&mut self, address: Address) {
        self.0.retain(|a| a != &address);
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
        Self(Vec::new()).put(session_id)
    }

    pub fn get_partial_keys<C: AppState>(
        &self,
        session_id: SessionId,
    ) -> Result<Vec<PartialKeySubmission<C::Signature, C::Address>>, KvStoreError> 
    where
        C::Address: From<Address>,
    {
        self
            .0
            .iter()
            .try_fold(Vec::new(), |mut acc, address| -> Result<Vec<PartialKeySubmission<C::Signature, C::Address>>, KvStoreError> {
                PartialKeySubmission::<C::Signature, C::Address>::get(session_id, C::Address::from(address.clone()))
                    .map(|partial_key_submission| {
                        acc.push(partial_key_submission);
                        acc
                    })
            })?
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
