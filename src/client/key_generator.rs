use std::sync::Arc;

use radius_sdk::json_rpc::client::{Id, RpcClient, RpcClientError};

use crate::{
    rpc::{
        cluster::{
            GetKeyGeneratorList, RunGeneratePartialKey, SyncAggregatedKey, SyncKeyGenerator,
            SyncPartialKey,
        },
        internal::AddDistributedKeyGeneration,
    },
    types::KeyGeneratorList,
};

pub struct DistributedKeyGenerationClient {
    inner: Arc<DistributedKeyGenerationClientInner>,
}

struct DistributedKeyGenerationClientInner {
    rpc_url: String,
    rpc_client: RpcClient,
}

impl Clone for DistributedKeyGenerationClient {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl DistributedKeyGenerationClient {
    pub fn new(rpc_url: impl AsRef<str>) -> Result<Self, RpcClientError> {
        let inner = DistributedKeyGenerationClientInner {
            rpc_url: rpc_url.as_ref().to_string(),
            rpc_client: RpcClient::new()?,
        };

        Ok(Self {
            inner: Arc::new(inner),
        })
    }

    pub async fn sync_partial_key(&self, parameter: SyncPartialKey) -> Result<(), RpcClientError> {
        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                SyncPartialKey::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await
    }

    pub async fn sync_key_generator(
        &self,
        parameter: AddDistributedKeyGeneration,
    ) -> Result<(), RpcClientError> {
        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                SyncKeyGenerator::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await
    }

    pub async fn get_key_generator_list(&self) -> Result<KeyGeneratorList, RpcClientError> {
        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                GetKeyGeneratorList::METHOD_NAME,
                &{},
                Id::Null,
            )
            .await
    }

    pub async fn run_generate_partial_key(
        &self,
        parameter: RunGeneratePartialKey,
    ) -> Result<(), RpcClientError> {
        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                RunGeneratePartialKey::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await
    }

    pub async fn sync_aggregated_key(
        &self,
        parameter: SyncAggregatedKey,
    ) -> Result<(), RpcClientError> {
        self.inner
            .rpc_client
            .request(
                &self.inner.rpc_url,
                SyncAggregatedKey::METHOD_NAME,
                &parameter,
                Id::Null,
            )
            .await
    }
}
