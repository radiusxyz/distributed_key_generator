use crate::EncKeyCommitment;

/// Event emitted by the key generator
#[derive(Debug, Clone)]
pub enum Event<Signature, Address> {
    /// There are enough partial keys to generate a decryption key
    ThresholdMet(Vec<EncKeyCommitment<Signature, Address>>),
}