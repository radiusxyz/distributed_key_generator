use std::sync::Arc;

use radius_sequencer_sdk::json_rpc::{Error, RpcClient};

use crate::rpc::cluster::SyncPartialKey;

pub struct KeyGeneratorClient {
    inner: Arc<RpcClient>,
}

impl KeyGeneratorClient {
    pub fn new(rpc_url: impl AsRef<str>) -> Result<Self, Error> {
        let rpc_client = RpcClient::new(rpc_url)?;

        Ok(Self {
            inner: Arc::new(rpc_client),
        })
    }

    pub async fn sync_partial_key(
        &self,
        parameter: SyncPartialKey,
    ) -> Result<(), Error> {
        self.inner
            .request(SyncPartialKey::METHOD_NAME, parameter)
            .await
    }
}
