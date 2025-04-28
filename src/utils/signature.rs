use ethers::{types::Signature as EthersSignature, utils::hash_message};
use radius_sdk::signature::{Address, PrivateKeySigner, Signature, SignatureError};
use serde::Serialize;

pub fn create_signature<T: Serialize>(
    signer: &PrivateKeySigner,
    message: &T,
) -> Result<Signature, SignatureError> {
    signer.sign_message(message)
}

pub fn verify_signature<T: Serialize>(
    signature: &Signature,
    message: &T,
) -> Result<Address, SignatureError> {
    let message_bytes = bincode::serialize(message).map_err(SignatureError::SerializeMessage)?;

    let message_hash = hash_message(message_bytes);

    let sig_bytes = signature.as_bytes();
    if sig_bytes.len() != 65 {
        return Err(SignatureError::UnsupportedChainType(
            "Invalid signature length".to_string(),
        ));
    }

    let mut sig_fixed = sig_bytes.to_vec();
    if sig_fixed[64] < 27 {
        sig_fixed[64] += 27;
    }

    let ethers_signature = EthersSignature::try_from(sig_fixed.as_slice())
        .map_err(|_| SignatureError::UnsupportedChainType("Signature parse failed".to_string()))?;

    let recovered_pubkey = ethers_signature.recover(message_hash).map_err(|_| {
        SignatureError::UnsupportedChainType("Signature recover failed".to_string())
    })?;

    Ok(Address::from(recovered_pubkey.as_bytes().to_vec()))
}

#[cfg(test)]
mod tests {
    use radius_sdk::signature::{ChainType, PrivateKeySigner};
    use serde::Serialize;

    use crate::utils::signature::{create_signature, verify_signature};

    #[derive(Debug, Serialize)]
    struct DummyMessage {
        field1: u32,
        field2: String,
    }

    #[test]
    fn test_create_and_verify_signature() {
        // Setup
        let private_key_hex = "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6";
        let signer = PrivateKeySigner::from_str(ChainType::Ethereum, private_key_hex).unwrap();

        let message = DummyMessage {
            field1: 42,
            field2: "hello".to_string(),
        };

        // Create signature
        let signature = create_signature(&signer, &message).expect("Failed to sign message");

        // Verify signature
        let recovered_address =
            verify_signature(&signature, &message).expect("Failed to verify signature");

        // Should match signer's address
        assert_eq!(&recovered_address, signer.address());
    }
}
