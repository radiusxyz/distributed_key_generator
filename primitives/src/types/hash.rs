use crate::Hasher;
use sha3::{digest::{ExtendableOutput, Update, XofReader}, Shake256, Sha3_256, Digest};

pub struct Sha3Hasher;

impl Hasher for Sha3Hasher {
    type Output = Vec<u8>;
    const LENGTH: usize = 32;

    fn hash(input: &[u8], maybe_size: Option<usize>) -> Self::Output {
        if let Some(size) = maybe_size {
            let mut hasher = Shake256::default();
            hasher.update(input);
            let mut reader = hasher.finalize_xof();
            let mut buf = vec![0u8; size];
            reader.read(&mut buf);
            buf
        } else {
            let mut hasher = Sha3_256::new();
            sha3::Digest::update(&mut hasher, input);
            let mut output = hasher.finalize().to_vec();
            output.truncate(Self::LENGTH);
            output
        }
    }
}