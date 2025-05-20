use crate::{FixedHasher, VariableHasher};
use sha2::{Digest, Sha256};
use sha3::{digest::{ExtendableOutput, Update, XofReader}, Shake256};
use skde::BigUint;

pub struct Shake256Hasher;

impl VariableHasher for Shake256Hasher {
    type Output = BigUint;

    fn hash(input: &[u8], size: usize) -> Self::Output {
        let mut hasher = Shake256::default();
        hasher.update(input);
        let mut reader = hasher.finalize_xof();
        let mut buf = vec![0u8; size];
        reader.read(&mut buf);
        BigUint::from_bytes_le(&buf)
    }
}

pub struct Sha256Hasher;

impl FixedHasher for Sha256Hasher {
    type Output = [u8; 32];

    const LENGTH: usize = 32;

    fn hash(input: &[u8]) -> Self::Output {
        let mut hasher = Sha256::new();
        sha2::Digest::update(&mut hasher, input);
        hasher.finalize().into()
    }
}