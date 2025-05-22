use std::marker::PhantomData;

use dkg_primitives::{
    AppState, DecKey, EncKey, Hasher, KeyGenerationError, SessionId, TraceExt, Error
};
use skde::{
    delay_encryption::{decrypt, encrypt, solve_time_lock_puzzle, SkdeParams},
    key_aggregation::{aggregate_key, AggregatedKey},
    key_generation::{generate_uv_pair, PartialKey},
    BigUint,
};
use tracing::info;

struct Skde<H> {
    params: SkdeParams,
    _phantom: PhantomData<H>,
}

impl<H: Hasher> Skde<H> 
where
    H::Output: AsRef<[u8]>,
{

    pub fn new(params: SkdeParams) -> Self {
        Self { params, _phantom: Default::default() }
    }

    pub fn get_enc_key(&self, session_id: SessionId, partial_key_list: &Vec<PartialKey>) -> Result<EncKey, Error> {
        let mut selected_keys = self.select_random_partial_keys(&partial_key_list, session_id)?;
        let derived_key = self.derive_partial_key(&selected_keys)?;
        selected_keys.push(derived_key);
        let enc_key: EncKey = aggregate_key(&self.params, &selected_keys).into();
        enc_key.put(session_id)?;
        Ok(enc_key)
    }

    pub fn get_dec_key(&self, session_id: SessionId, enc_key: &AggregatedKey) -> Result<DecKey, Error> {
        // TODO: Timeout
        let secure_key = solve_time_lock_puzzle(&self.params, enc_key).map_err(|err| {
            KeyGenerationError::InternalError(format!("Failed to solve time lock puzzle: {:?}", err))
        })?;
    
        let dec_key = DecKey::new(secure_key.sk.clone());
    
        dec_key.put(session_id).map_err(|err| {
            KeyGenerationError::InvalidPartialKey(format!("Failed to store decryption key: {:?}", err))
        })?;
    
        Ok(dec_key)
    }

    pub fn verify_key_pair(&self, encryption_key: &str, decryption_key: &str) -> Result<(), KeyGenerationError> {
        let sample_message = "sample_message";

        let ciphertext = encrypt(&self.params, sample_message, encryption_key, true)
            .ok_or_trace()
            .ok_or_else(|| KeyGenerationError::InternalError("Encryption failed".into()))?;

        let decrypted_message = match decrypt(&self.params, &ciphertext, decryption_key) {
            Ok(message) => message,
            Err(err) => {
                tracing::error!("Decryption failed: {}", err);
                return Err(KeyGenerationError::InternalError(
                    format!("Decryption failed: {}", err).into(),
                ));
            }
        };

        if decrypted_message.as_str() != sample_message {
            return Err(KeyGenerationError::InternalError(
                "Decryption failed: message mismatch".into(),
            ));
        }

        Ok(())
    }

    fn get_randomness(&self, session_id: SessionId) -> Vec<u8> {
        match session_id.prev() {
            Some(prev) => match DecKey::get(prev) {
                Ok(key) => key.into(),
                Err(_) => b"default-randomness".to_vec(),
            },
            None => {
                // Underflow means `initial session`
                return b"initial-randomness".to_vec();
            }
        }
    }

    fn select_ordered_indices(&self, n: usize, session_id: SessionId) -> Result<Vec<usize>, Error> {
        assert!(n >= 1, "Need at least 1 partial key to proceed");

        // Special case: when there is only one partial key, return index 0
        // Having only one key is insecure and should not be used in production
        if n == 1 {
            return Ok(vec![0]); 
        }
        let randomness = self.get_randomness(session_id);
        let first_byte = randomness[0] as usize;
        // TODO: k must be less than maximum number of partial key generators
        let k = (first_byte % (n - 1)) + 1;

        let mut indices: Vec<usize> = (0..n).collect();
        let mut state = randomness.to_vec();

        for i in (1..n).rev() {
            let mut input = Vec::with_capacity(state.len() + 1);
            input.extend_from_slice(&state);
            input.push(i as u8);
            let hash = H::hash(&input, None);
            let output: [u8; 32] = hash.as_ref().try_into().map_err(|_| Error::ConvertError("Failed to convert hash to 32 bytes".into()))?;
            let rand_byte = u64::from_le_bytes(output[0..8].try_into().unwrap());
            let j = (rand_byte % (i as u64 + 1)) as usize;
            indices.swap(i, j);
            state = output.to_vec();
        }

        Ok(indices[..k].to_vec())
    }

    fn select_random_partial_keys(&self, partial_keys: &Vec<PartialKey>, session_id: SessionId) -> Result<Vec<PartialKey>, Error> {
        let indices = self.select_ordered_indices(partial_keys.len(), session_id)?;
        Ok(indices.iter().map(|&i| partial_keys[i].clone()).collect())
    }

    fn derive_partial_key(&self, selected_keys: &Vec<PartialKey>) -> Result<PartialKey, Error> {
        let n = BigUint::parse_bytes(self.params.n.as_bytes(), 10).unwrap();
        let max_sequencer_number =
            BigUint::parse_bytes(self.params.max_sequencer_number.as_bytes(), 10).unwrap();

        // Creates a virtual partial key based on hashed combination
        let mut h_input = Vec::new();
        for key in selected_keys {
            h_input.extend(serde_json::to_vec(key).unwrap());
        }

        let n_half = &n / 2u32;
        let n_half_over_max_sequencer_number = &n / (2u32 * &max_sequencer_number);

        let gen = |label: &[u8]| -> Result<BigUint, Error> {
            let mut input = h_input.clone();
            input.push(label[0]);
            let hash = H::hash(&input, Some(32));
            let hash: [u8; 32] = hash.as_ref().try_into().map_err(|_| Error::ConvertError("Failed to convert hash to 32 bytes".into()))?;
            Ok(BigUint::from_bytes_le(&hash))
        };

        let r_h = gen(b"r")? % &n_half_over_max_sequencer_number;
        let s_h = gen(b"s")? % &n_half_over_max_sequencer_number;
        let k_h = gen(b"k")? % &n_half;
        // u, v = g^(r + s), h^{(r + s) * n} * (1 + n)^s
        let uv_pair = generate_uv_pair(&self.params, &(&r_h + &s_h), &s_h)
            .expect("Failed to generate UV pair for partial key");

        // y, w = g^k, g^{k * n} * (1 + n)^r
        let yw_pair =
            generate_uv_pair(&self.params, &k_h, &r_h).expect("Failed to generate YW pair for partial key");

        Ok(PartialKey {u: uv_pair.u, v: uv_pair.v, y: yw_pair.u, w: yw_pair.v})
    }
}



