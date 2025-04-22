use std::time::{SystemTime, UNIX_EPOCH};

use radius_sdk::signature::Address;
use skde::{
    delay_encryption::SkdeParams,
    key_generation::{
        generate_partial_key, prove_partial_key_validity, PartialKey as SkdePartialKey,
        PartialKeyProof,
    },
};

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// A extension method to convert an address to a short format
pub trait AddressExt {
    fn to_short(&self) -> String;
}

impl AddressExt for Address {
    fn to_short(&self) -> String {
        let hex = self.as_hex_string();
        if hex.len() < 10 {
            hex
        } else {
            format!("{}", &hex[..6])
        }
    }
}

/// Create a new partial key with its validity proof
pub fn generate_partial_key_with_proof(
    skde_params: &SkdeParams,
) -> (SkdePartialKey, PartialKeyProof) {
    // Generate the partial key using the SKDE library
    let (secret_value, skde_partial_key) =
        generate_partial_key(skde_params).expect("Failed to generate partial key");

    // Generate proof of validity for the key
    let proof = prove_partial_key_validity(skde_params, &secret_value)
        .expect("Failed to generate proof for partial key");

    (skde_partial_key, proof)
}
