use std::time::{SystemTime, UNIX_EPOCH};

use radius_sdk::{
    self,
    json_rpc::server::RpcError,
    signature::{Address, Signature},
};
use serde::Serialize;

/// Generates the current timestamp in seconds since UNIX epoch
pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn create_signature<T: Serialize>(_message: &T) -> Signature {
    Signature::from(vec![0; 64])
}

pub fn verify_signature<T: Serialize>(
    _signature: &Signature,
    _message: &T,
    // context: &AppState,
) -> Result<Address, RpcError> {
    // let signature = generate_dummy_signature();
    // let message_bytes = serialize_to_bincode(message)?;
    let dummy_signer_address = Address::from(vec![0; 20]);

    Ok(dummy_signer_address)
}
