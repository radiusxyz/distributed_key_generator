use radius_sdk::{
    self,
    json_rpc::server::RpcError,
    signature::{Address, Signature},
};
use serde::Serialize;

pub fn create_signature<T: Serialize>(_message: &T) -> Signature {
    let signature = Signature::from(vec![0; 64]);
    signature
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
