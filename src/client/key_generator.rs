use std::sync::Arc;

use radius_sequencer_sdk::json_rpc::{Error, RpcClient};

use crate::{
    rpc::{
        cluster::{GetKeyGeneratorList, RunGeneratePartialKey, SyncKeyGenerator, SyncPartialKey},
        internal::AddKeyGenerator,
    },
    types::KeyGeneratorList,
};

#[derive(Clone)]
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

    pub async fn sync_partial_key(&self, parameter: SyncPartialKey) -> Result<(), Error> {
        self.inner
            .request(SyncPartialKey::METHOD_NAME, parameter)
            .await
    }

    pub async fn sync_key_generator(&self, parameter: AddKeyGenerator) -> Result<(), Error> {
        self.inner
            .request(SyncKeyGenerator::METHOD_NAME, parameter)
            .await
    }

    pub async fn get_key_generator_list(&self) -> Result<KeyGeneratorList, Error> {
        self.inner
            .request(GetKeyGeneratorList::METHOD_NAME, {})
            .await
    }

    pub async fn run_generate_partial_key(
        &self,
        parameter: RunGeneratePartialKey,
    ) -> Result<(), Error> {
        self.inner
            .request(RunGeneratePartialKey::METHOD_NAME, parameter)
            .await
    }
}
