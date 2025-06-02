use crate::EncKeyCommitment;

/// Event of this node
#[derive(Debug, Clone)]
pub enum Event<Signature, Address> {
    /// There are enough encryption keys to generate a decryption key
    ThresholdMet(Vec<EncKeyCommitment<Signature, Address>>),
}