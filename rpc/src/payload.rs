use dkg_primitives::{EncKeyCommitment, SessionId, SignedCommitment};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A payload for submitting a finalized encryption key to the network
/// Payload is a vector of signed commitments from the committee members
/// Contains vec of `EncKeyPayload`
pub struct FinalizedEncKeyPayload<Signature, Address> {
    pub randomness: Vec<u8>,
    /// The encryption keys
    pub enc_keys: Vec<EncKeyCommitment<Signature, Address>>,
}

impl<Signature: Clone, Address: Clone> FinalizedEncKeyPayload<Signature, Address> {
    pub fn new(
        randomness: Vec<u8>,
        commitments: Vec<EncKeyCommitment<Signature, Address>>,
    ) -> Self {
        Self {
            randomness,
            enc_keys: commitments,
        }
    }

    pub fn len(&self) -> usize {
        self.enc_keys.len()
    }

    pub fn randomness(&self) -> Vec<u8> {
        self.randomness.clone()
    }

    pub fn enc_keys(&self) -> Vec<EncKeyCommitment<Signature, Address>> {
        self.enc_keys.clone()
    }

    pub fn concat(&self) -> Vec<u8> {
        self.enc_keys
            .iter()
            .map(|c| c.inner().commitment.payload.inner())
            .flatten()
            .collect()
    }
}

impl<Signature, Address> IntoIterator for FinalizedEncKeyPayload<Signature, Address> {
    type Item = EncKeyCommitment<Signature, Address>;
    type IntoIter = std::vec::IntoIter<EncKeyCommitment<Signature, Address>>;

    fn into_iter(self) -> Self::IntoIter {
        self.enc_keys.into_iter()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncKeyPayload(Vec<u8>);

impl EncKeyPayload {
    pub fn new(enc_key: Vec<u8>) -> Self {
        Self(enc_key)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A payload for submitting a decryption key to the network
pub struct DecKeyPayload {
    /// Finalized encryption key
    pub enc_key: Vec<u8>,
    /// Decryption key
    pub dec_key: Vec<u8>,
    /// Randomness used to generate the decryption key
    pub randomness: Vec<u8>,
    /// The timestamp at which the decryption key was solved
    pub solve_at: u128,
}

impl DecKeyPayload {
    pub fn new(enc_key: Vec<u8>, dec_key: Vec<u8>, randomness: Vec<u8>, solve_at: u128) -> Self {
        Self {
            enc_key,
            dec_key,
            randomness,
            solve_at,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealPayload<Signature, Address> {
    pub session_id: SessionId,
    pub enc_commitments: Vec<SignedCommitment<Signature, Address>>,
    pub dec_commitment: SignedCommitment<Signature, Address>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HeartbeatPayload<Address> {
    pub address: Address,
    pub at: SessionId,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StartTimePayload(u128);

impl StartTimePayload {
    pub fn new(start_time: u128) -> Self {
        Self(start_time)
    }

    pub fn start_time(&self) -> u128 {
        self.0
    }
}
