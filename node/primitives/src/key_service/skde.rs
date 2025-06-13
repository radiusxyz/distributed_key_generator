use super::KeyServiceResult;
use std::marker::PhantomData;
use dkg_primitives::{Hasher, KeyService, KeyServiceError};
use skde::{
    delay_encryption::{decrypt, encrypt, solve_time_lock_puzzle, SkdeParams},
    key_aggregation::{aggregate_key, AggregatedKey},
    key_generation::{generate_partial_key, generate_uv_pair, PartialKey},
    BigUint,
};
use dkg_utils::timestamp;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skde<H> {
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

    pub fn gen_enc_key(&self, randomness: Vec<u8>, maybe_enc_keys: Option<Vec<Vec<u8>>>) -> KeyServiceResult<Vec<u8>> {
        // If enc_keys are provided, aggregate them
        let enc_key = if let Some(enc_keys) = maybe_enc_keys {
            // Try deserialize enc_keys into PartialKey, error if failed to deserialize
            let partial_keys = enc_keys.iter().map(|key| serde_json::from_slice::<PartialKey>(key).map_err(KeyServiceError::from)).collect::<Result<Vec<PartialKey>, KeyServiceError>>()?;
            let mut selected_keys = self.select_random_partial_keys(&partial_keys, randomness)?;
            let derived_key = self.derive_partial_key(&selected_keys)?;
            selected_keys.push(derived_key);
            let enc_key = aggregate_key(&self.params, &selected_keys);
            serde_json::to_vec(&enc_key)?
        } else {
            let (_, partial_key) = generate_partial_key(&self.params).map_err(|e| KeyServiceError::InternalError(e.to_string()))?;
            serde_json::to_vec(&partial_key)?
        };
        Ok(enc_key)
    }

    pub fn gen_dec_key(&self, enc_key: &Vec<u8>) -> KeyServiceResult<(Vec<u8>, u128)> {
        // TODO: Timeout
        let enc_key = serde_json::from_slice::<AggregatedKey>(enc_key)?;
        let secure_key = solve_time_lock_puzzle(&self.params, &enc_key).map_err(|e| KeyServiceError::InternalError(e.to_string()))?;
        Ok((serde_json::to_vec(&secure_key.sk)?, timestamp()))
    }

    pub fn verify_dec_key(&self, enc_key: &Vec<u8>, dec_key: &Vec<u8>) -> KeyServiceResult<()> {
        let sample_message = "sample_message";
        let enc_key = serde_json::from_slice::<AggregatedKey>(enc_key)?;
        let dec_key = serde_json::from_slice::<String>(dec_key)?;
        let ciphertext = encrypt(&self.params, sample_message, &enc_key.u, true)?;
        let decrypted_message = decrypt(&self.params, &ciphertext, &dec_key)?;
        if decrypted_message.as_str() != sample_message { return Err(KeyServiceError::MessageMismatch); }
        Ok(())
    }

    fn select_ordered_indices(&self, n: usize, randomness: Vec<u8>) -> KeyServiceResult<Vec<usize>> {
        assert!(n >= 1, "Need at least 1 partial key to proceed");

        // Special case: when there is only one partial key, return index 0
        // Having only one key is insecure and should not be used in production
        if n == 1 {
            return Ok(vec![0]); 
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
            let hash = H::hash(&input, None);
            let output: [u8; 32] = hash.as_ref().try_into().map_err(|_| KeyServiceError::InternalError("Failed to convert hash to 32 bytes".into()))?;
            let rand_byte = u64::from_le_bytes(output[0..8].try_into().unwrap());
            let j = (rand_byte % (i as u64 + 1)) as usize;
            indices.swap(i, j);
            state = output.to_vec();
        }

        Ok(indices[..k].to_vec())
    }

    fn select_random_partial_keys(&self, partial_keys: &Vec<PartialKey>, randomness: Vec<u8>) -> KeyServiceResult<Vec<PartialKey>> {
        let indices = self.select_ordered_indices(partial_keys.len(), randomness)?;
        Ok(indices.iter().map(|&i| partial_keys[i].clone()).collect())
    }

    fn derive_partial_key(&self, selected_keys: &Vec<PartialKey>) -> KeyServiceResult<PartialKey> {
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

        let gen = |label: &[u8]| -> KeyServiceResult<BigUint> {
            let mut input = h_input.clone();
            input.push(label[0]);
            let hash = H::hash(&input, Some(32));
            let hash: [u8; 32] = hash.as_ref().try_into().map_err(|_| KeyServiceError::InternalError("Failed to convert hash to 32 bytes".into()))?;
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

impl<H> KeyService for Skde<H> 
where 
    H: Hasher,
    H::Output: AsRef<[u8]>,
{
    type TrustedSetUp = SkdeParams;
    type Metadata = Vec<PartialKey>;
    type Error = KeyServiceError; 

    fn setup(param: SkdeParams) -> Self {
        Skde::<H>::new(param)
    }

    fn get_trusted_setup(&self) -> SkdeParams {
        self.params.clone()
    }

    fn gen_enc_key(&self, randomness: Vec<u8>, maybe_enc_keys: Option<Vec<Vec<u8>>>) -> KeyServiceResult<Vec<u8>> {
        self.gen_enc_key(randomness, maybe_enc_keys)
    }

    fn gen_dec_key(&self, enc_key: &Vec<u8>) -> KeyServiceResult<(Vec<u8>, u128)> {
        self.gen_dec_key(enc_key)
    }

    fn verify_dec_key(&self, enc_key: &Vec<u8>, dec_key: &Vec<u8>) -> KeyServiceResult<()> {
        self.verify_dec_key(enc_key, dec_key)?;
        Ok(())
    }
}



