use crate::PartialKeySubmission;

/// Event emitted by the key generator
#[derive(Debug, Clone)]
pub enum Event<Signature, Address> {
    /// There are enough partial keys to generate a decryption key
    ThresholdMet(Vec<PartialKeySubmission<Signature, Address>>),
}