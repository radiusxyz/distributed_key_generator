
use crate::primitives::{Deserialize, Serialize, Address};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGeneratorMessage {
    address: Address,
    cluster_rpc_url: String,
    external_rpc_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGenerator {
    // signature: Signature, // TODO: Uncomment this code
    message: AddKeyGeneratorMessage,
}