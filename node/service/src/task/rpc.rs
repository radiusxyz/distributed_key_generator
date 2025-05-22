use dkg_primitives::AppState;
use radius_sdk::json_rpc::server::RpcServer;
use dkg_rpc::{SyncKeyGenerator, SyncPartialKey, SyncFinalizedPartialKeys, SyncDecKey, AddKeyGenerator, GetKeyGeneratorList, GetEncKey, GetDecKey, GetSessionId, GetSkdeParams, GetHealth};

/// Configure the cluster RPC server.
pub async fn default_cluster_rpc_server<C: AppState>(ctx: &mut C) -> Result<RpcServer<C>, C::Error> {
    RpcServer::new(ctx.clone())
        .register_rpc_method::<SyncKeyGenerator<C::Address>>()?
        .register_rpc_method::<SyncPartialKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SyncFinalizedPartialKeys<C::Signature, C::Address>>()?
        .register_rpc_method::<SyncDecKey<C::Signature, C::Address>>()
        .map_err(C::Error::from)
}

/// Configure the external RPC server.
pub async fn default_external_rpc_server<C: AppState>(ctx: &C) -> Result<RpcServer<C>, C::Error> {
    RpcServer::new(ctx.clone())
        .register_rpc_method::<AddKeyGenerator<C::Address>>()?
        .register_rpc_method::<GetKeyGeneratorList>()?
        .register_rpc_method::<GetDecKey>()?
        .register_rpc_method::<GetEncKey>()?
        .register_rpc_method::<GetSessionId>()?
        .register_rpc_method::<GetSkdeParams>()?
        .register_rpc_method::<GetHealth>()
        .map_err(C::Error::from)
}
