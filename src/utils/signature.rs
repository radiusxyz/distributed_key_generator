use ethers::{types::Signature as EthersSignature, utils::keccak256};
use radius_sdk::signature::{Address, Signature, SignatureError};
use serde::Serialize;

use crate::AppState;

pub fn create_signature<T: Serialize>(
    context: &AppState,
    message: &T,
) -> Result<Signature, SignatureError> {
    context.config().signer().sign_message(message)
}

pub fn verify_signature<T: Serialize>(
    signature: &Signature,
    message: &T,
) -> Result<Address, SignatureError> {
    let message_bytes = bincode::serialize(message).map_err(SignatureError::SerializeMessage)?;

    let sig_bytes = signature.as_bytes();
    if sig_bytes.len() != 65 {
        return Err(SignatureError::UnsupportedChainType(
            "Invalid signature length".to_string(),
        ));
    }

    // Fix v (last byte) if needed (Ethers expects v = 27 or 28)
    let mut sig_fixed = sig_bytes.to_vec();
    if sig_fixed[64] < 27 {
        sig_fixed[64] += 27;
    }

    let message_hash = keccak256(message_bytes);

    // Parse signature
    let ethers_signature = EthersSignature::try_from(sig_fixed.as_slice())
        .map_err(|_| SignatureError::UnsupportedChainType("Signature parse failed".to_string()))?;

    // Recover signer address
    let recovered_pubkey = ethers_signature.recover(message_hash).map_err(|_| {
        SignatureError::UnsupportedChainType("Signature recover failed".to_string())
    })?;

    Ok(Address::from(recovered_pubkey.as_bytes().to_vec()))
}
