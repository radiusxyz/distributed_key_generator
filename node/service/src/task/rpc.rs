use dkg_primitives::AppState;
use radius_sdk::json_rpc::server::RpcServer;
use tracing::info;
use dkg_rpc::{
    internal::AddKeyGenerator,
    cluster::{
        GetKeyGeneratorList, 
        SyncKeyGenerator, 
        SyncPartialKey, 
        ClusterSyncFinalizedPartialKeys, 
        SyncDecryptionKey, 
        SubmitPartialKey, 
        RequestSubmitPartialKey,
    },
    external::{
        GetEncryptionKey,
        GetDecryptionKey,
        GetLatestEncryptionKey,
        GetLatestSessionId,
        GetSkdeParams,
    },
    solver::{SubmitDecryptionKey, SolverSyncFinalizedPartialKeys},
};

/// Initialize the internal RPC server.
pub async fn initialize_internal_rpc_server<C: AppState>(ctx: &C, url: &str) -> Result<(), C::Error> {
    let internal_rpc_server = RpcServer::new(ctx.clone())
        .register_rpc_method::<AddKeyGenerator<C::Address>>()?
        .init(url)
        .await
        .map_err(C::Error::from)?;

    info!(
        "{} Internal RPC server runs at {}",
        ctx.log_prefix(), url
    );

    ctx.spawn_task(Box::pin(async move {
        internal_rpc_server.stopped().await;
    }));

    Ok(())
}

/// Initialize the cluster RPC server.
pub async fn initialize_cluster_rpc_server<C: AppState>(ctx: &C, url: &str) -> Result<(), C::Error> {
    let key_generator_rpc_server = RpcServer::new(ctx.clone())
        .register_rpc_method::<GetKeyGeneratorList>()?
        .register_rpc_method::<SyncKeyGenerator<C::Address>>()?
        .register_rpc_method::<SyncPartialKey<C::Signature, C::Address>>()?
        .register_rpc_method::<ClusterSyncFinalizedPartialKeys<C::Signature, C::Address>>()?
        .register_rpc_method::<SyncDecryptionKey<C::Signature, C::Address>>()?
        .register_rpc_method::<SubmitPartialKey<C::Signature, C::Address>>()?
        .register_rpc_method::<RequestSubmitPartialKey>()?
        .init(url)
        .await
        .map_err(C::Error::from)?;

    info!(
        "{} Cluster RPC server runs at {}",
        ctx.log_prefix(), url
    );

    ctx.spawn_task(Box::pin(async move {
        key_generator_rpc_server.stopped().await;
    }));

    Ok(())
}

/// Initialize the external RPC server.
pub async fn initialize_external_rpc_server<C: AppState>(ctx: &C, url: &str) -> Result<(), C::Error> {
    let external_rpc_server = RpcServer::new(ctx.clone())
        .register_rpc_method::<GetEncryptionKey>()?
        .register_rpc_method::<GetDecryptionKey>()?
        .register_rpc_method::<GetLatestEncryptionKey>()?
        .register_rpc_method::<GetLatestSessionId>()?
        .register_rpc_method::<GetSkdeParams>()?
        .init(url)
        .await
        .map_err(C::Error::from)?;

    info!(
        "{} External RPC server runs at {}",
        ctx.log_prefix(), url
    );

    ctx.spawn_task(Box::pin(async move {
        external_rpc_server.stopped().await;
    }));

    Ok(())
}

async fn initialize_solver_rpc_server<C: AppState>(ctx: &C, url: &str) -> Result<(), C::Error> {
    let prefix = ctx.log_prefix();
    let rpc_server_builder = RpcServer::new(ctx.clone());
    let rpc_server = if ctx.is_leader() {
        rpc_server_builder
            .register_rpc_method::<GetSkdeParams>()?
            .register_rpc_method::<SubmitDecryptionKey<C::Signature, C::Address>>()?
    } else {
        rpc_server_builder.register_rpc_method::<SolverSyncFinalizedPartialKeys<C::Signature, C::Address>>()?
    };

    let rpc_server = rpc_server
        .init(url)
        .await
        .map_err(C::Error::from)?;

    info!("{} Solver RPC server runs at {}", prefix, url);

    ctx.spawn_task(Box::pin(async move {
        rpc_server.stopped().await;
    }));

    Ok(())
}
