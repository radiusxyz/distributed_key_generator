use dkg_primitives::{DecKey, SessionId, Error, Selector, Sha256Hasher, FixedHasher, Aggregator};
use skde::{delay_encryption::SkdeParams, key_generation::PartialKey, key_aggregation::{AggregatedKey, aggregate_key}};

pub struct DkgAggregator;

impl Aggregator<SessionId, Error> for DkgAggregator {
    type PartialKey = PartialKey;
    type AggregatedKey = AggregatedKey;
    type Selector = DkgRandomness;

    fn finalize(&self, session_id: SessionId, partial_keys: Vec<Self::PartialKey>) -> Result<Self::AggregatedKey, Error> {
        let indices = Self::Selector::select(partial_keys.len(), session_id);
        let selected_keys: Vec<Self::PartialKey> = indices.iter().map(|&i| partial_keys[i].clone()).collect();
        let derived_key = self.derive_partial_key(&selected_keys, &skde_params);
        selected_keys.push(derived_key);
        Ok(aggregate_key(&skde_params, &selected_keys))
    }

    fn derive_partial_key(&self, selected_keys: &Vec<Self::PartialKey>, params: &SkdeParams) -> Self::PartialKey {}

    fn calculate_decryption_key<Hasher>(partial_keys: Vec<Self::PartialKey>) -> Result<String, Error> {}
}

pub struct DkgRandomness;

impl Selector<SessionId> for DkgRandomness {
    
    type Hasher = Sha256Hasher;

    fn get_randomness(session_id: SessionId) -> Vec<u8> {
        match session_id.prev() {
            Some(prev) => match DecKey::get(prev) {
                Ok(key) => key.to_bytes(),
                Err(_) => b"default-randomness".to_vec(),
            },
            None => {
                // Underflow means `initial session`
                return b"initial-randomness".to_vec();
            }
        }
    }

    fn select(n: usize, session_id: SessionId) -> Vec<usize> {
        let randomness = Self::get_randomness(session_id);
        assert!(n >= 1, "Need at least 1 partial key to proceed");

        // Special case: when there is only one partial key, return index 0
        // Having only one key is insecure and should not be used in production
        if n == 1 {
            return vec![0];
        }

        let first_byte = randomness[0] as usize;
        // TODO: k must be less than maximum number of partial key generators
        let k = (first_byte % (n - 1)) + 1;

        let mut indices: Vec<usize> = (0..n).collect();
        let mut state = randomness.to_vec();

        for i in (1..n).rev() {
            let mut input = Vec::with_capacity(state.len() + 1);
            input.extend_from_slice(&state);
            input.push(i as u8);
            let hash = Self::Hasher::hash(&input);
            let rand_byte = u64::from_le_bytes(hash[0..8].try_into().unwrap());
            let j = (rand_byte % (i as u64 + 1)) as usize;
            indices.swap(i, j);
            state = hash.to_vec();
        }

        indices[..k].to_vec()
    }
}