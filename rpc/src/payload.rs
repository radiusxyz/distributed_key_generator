use serde::{Deserialize, Serialize};
use dkg_primitives::{SignedCommitment, SessionId, EncKeyCommitment};

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A payload for submitting a finalized encryption key to the network
/// Payload is a vector of signed commitments from the committee members 
/// Contains vec of `EncKeyPayload`
pub struct FinalizedEncKeyPayload<Signature, Address>(Vec<EncKeyCommitment<Signature, Address>>);

impl<Signature: Clone, Address: Clone> FinalizedEncKeyPayload<Signature, Address> {
    pub fn new(commitments: Vec<EncKeyCommitment<Signature, Address>>) -> Self { Self(commitments) }

    pub fn len(&self) -> usize { self.0.len() }

    pub fn inner(&self) -> Vec<EncKeyCommitment<Signature, Address>> { self.0.clone() }
}

impl<Signature, Address> IntoIterator for FinalizedEncKeyPayload<Signature, Address> {
    type Item = EncKeyCommitment<Signature, Address>;
    type IntoIter = std::vec::IntoIter<EncKeyCommitment<Signature, Address>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncKeyPayload(Vec<u8>);

impl EncKeyPayload {
    pub fn new(enc_key: Vec<u8>) -> Self { Self(enc_key) }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A payload for submitting a decryption key to the network
pub struct DecKeyPayload{
    /// Decryption key 
    pub dec_key: Vec<u8>,
    /// The timestamp at which the decryption key was solved
    pub solve_at: u128,
}

impl DecKeyPayload {
    pub fn new(dec_key: Vec<u8>, solve_at: u128) -> Self {
        Self { dec_key, solve_at }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FinalRevealPayload<Signature, Address> {
    pub session_id: SessionId,
    pub enc_commitments: Vec<SignedCommitment<Signature, Address>>,
    pub dec_commitment: SignedCommitment<Signature, Address>,
}
