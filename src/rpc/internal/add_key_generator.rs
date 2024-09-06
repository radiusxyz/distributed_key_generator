use crate::{models::KeyGeneratorModel, rpc::prelude::*};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AddKeyGenerator {
    address: Address,
    ip_address: String,
}

impl AddKeyGenerator {
    pub const METHOD_NAME: &'static str = "add_key_generator";

    pub async fn handler(parameter: RpcParameter, context: Arc<AppState>) -> Result<(), RpcError> {
        let parameter = parameter.parse::<Self>()?;

        let key_generator = KeyGenerator::new(parameter.address, parameter.ip_address);

        KeyGeneratorModel::put(&key_generator)?;

        context.add_key_generator_client(key_generator).await;

        Ok(())
    }
}
