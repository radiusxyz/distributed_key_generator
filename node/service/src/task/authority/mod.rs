
use dkg_primitives::AsyncTask;
use dkg_rpc::external::GetTrustedSetup;
use tracing::info;
use tokio::task::JoinHandle;
use super::{RpcServer, AppState, Config, Error};

pub async fn run_node<C: AppState>(ctx: &mut C, config: Config) -> Result<Vec<JoinHandle<()>>, Error> {
    let rpc_server = RpcServer::new(ctx.clone())
        .register_rpc_method::<GetTrustedSetup>()?
        .init(config.external_rpc_url.clone())
        .await
        .map_err(Error::from)?;

    info!("RPC server runs at {}", config.external_rpc_url);

    let handle = ctx.async_task().spawn_task(async move { rpc_server.stopped().await; });

    Ok(vec![handle])
}