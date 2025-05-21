use std::{fmt::{Debug, Display}, time::{SystemTime, UNIX_EPOCH}};
use serde::{Deserialize, Serialize};
use skde::key_generation::PartialKey;

use crate::{PartialKeySubmission, SessionId};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncFinalizedPartialKeysPayload<Signature, Address> {
    pub sender: Address,
    pub partial_key_submissions: Vec<PartialKeySubmission<Signature, Address>>,
    pub session_id: SessionId,
    pub ack_timestamp: u128,
}

impl<Signature: Clone, Address: Clone> SyncFinalizedPartialKeysPayload<Signature, Address> {
    pub fn new(sender: Address, partial_key_submissions: Vec<PartialKeySubmission<Signature, Address>>, session_id: SessionId) -> Self {
        let ack_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, partial_key_submissions, session_id, ack_timestamp }
    }

    pub fn len(&self) -> usize {
        self.partial_key_submissions.len()
    }

    /// Get sorted partial keys
    pub fn partial_keys(&self) -> Vec<PartialKeySubmission<Signature, Address>> {
        let mut sorted = self.partial_key_submissions.clone();
        sorted.sort_by(|a, b| a.payload.partial_key.u.cmp(&b.payload.partial_key.u));
        sorted
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartialKeyPayload<Address> {
    pub sender: Address,
    pub partial_key: PartialKey,
    pub submit_timestamp: u128,
    pub session_id: SessionId,
}

impl<Address> PartialKeyPayload<Address> {
    pub fn new(sender: Address, partial_key: PartialKey, session_id: SessionId) -> Self {
        let submit_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, partial_key, submit_timestamp, session_id }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncPartialKeyPayload<Signature, Address> {
    sender: Address,
    pub partial_key_submission: PartialKeySubmission<Signature, Address>,
    pub session_id: SessionId,
    pub ack_timestamp: u128,
}

impl<Signature, Address> SyncPartialKeyPayload<Signature, Address> {
    pub fn new(sender: Address, partial_key_submission: PartialKeySubmission<Signature, Address>) -> Self {
        let ack_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let session_id = partial_key_submission.payload.session_id;
        Self { sender, partial_key_submission, session_id, ack_timestamp }
    }

    pub fn sender(&self) -> &Address {
        &self.sender
    }

    pub fn partial_key_sender(&self) -> &Address {
        &self.partial_key_submission.payload.sender
    }
}

impl<Signature, Address: Debug> Display for SyncPartialKeyPayload<Signature, Address> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Received partial key ACK - sender:{:?}, session_id: {:?}, timestamp: {}", 
            self.sender, 
            self.session_id, 
            self.ack_timestamp
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SubmitDecryptionKeyPayload<Address> {
    pub sender: Address,
    pub decryption_key: String,
    pub session_id: SessionId,
    pub timestamp: u128,
}

impl<Address> SubmitDecryptionKeyPayload<Address> {
    pub fn new(sender: Address, decryption_key: String, session_id: SessionId) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, decryption_key, session_id, timestamp }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SyncDecryptionKeyPayload<Address> {
    pub sender: Address,
    pub decryption_key: String,
    pub session_id: SessionId,
    pub solve_timestamp: u128,
    pub ack: u128,
}

impl<Address> SyncDecryptionKeyPayload<Address> {
    pub fn new(sender: Address, decryption_key: String, session_id: SessionId, solve_timestamp: u128) -> Self {
        let ack = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { sender, decryption_key, session_id, solve_timestamp, ack }
    }
}
