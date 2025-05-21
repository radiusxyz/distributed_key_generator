use dkg_primitives::AppState;
use radius_sdk::json_rpc::server::RpcServer;
use dkg_rpc::{
    SyncKeyGenerator, 
    SyncPartialKey, 
    SyncFinalizedPartialKeys, 
    SyncDecryptionKey, 
    SubmitPartialKey, 
    RequestSubmitPartialKey,
    AddKeyGenerator,
    GetKeyGeneratorList,
    GetEncryptionKey,
    GetDecryptionKey,
    GetLatestEncryptionKey,
    GetLatestSessionId,
    GetSkdeParams,
};

/// Initialize the cluster  RPC server.
pub async fn default_cluster_rpc_server<C: AppState>(ctx: &mut C) -> Result<RpcServer<C>, C::Error> {
    RpcServer::new(ctx.clone())
        .register_rpc_method::<SyncKeyGenerator<C::Address>>()?
        .register_rpc_method::<SyncPartialKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SyncFinalizedPartialKeys<C::Signature, C::Address>>()?
        .register_rpc_method::<SyncDecryptionKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SubmitPartialKey<C::Signature, C::Address>>()?
        .register_rpc_method::<RequestSubmitPartialKey>()
        .map_err(C::Error::from)
}

/// Initialize the external RPC server.
pub async fn default_external_rpc_server<C: AppState>(ctx: &C) -> Result<RpcServer<C>, C::Error> {
    RpcServer::new(ctx.clone())
        .register_rpc_method::<AddKeyGenerator<C::Address>>()?
        .register_rpc_method::<GetKeyGeneratorList>()?
        .register_rpc_method::<GetEncryptionKey>()?
        .register_rpc_method::<GetDecryptionKey>()?
        .register_rpc_method::<GetLatestEncryptionKey>()?
        .register_rpc_method::<GetLatestSessionId>()?
        .register_rpc_method::<GetSkdeParams>()
        .map_err(C::Error::from)
}
