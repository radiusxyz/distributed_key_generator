use std::{fs, path::PathBuf};
use dkg_primitives::{AsyncTask, TrustedSetupFor, SecureBlock};
use dkg_rpc::external::GetTrustedSetup;
use tracing::info;
use tokio::task::JoinHandle;
use super::{RpcServer, AppState, Config, Error};
use crate::SignedTrustedSetup;

pub async fn run_node<C: AppState>(ctx: &mut C, config: Config) -> Result<Vec<JoinHandle<()>>, Error> {
    let skde_path = run_setup_trusted_setup::<C>(ctx, &config);
    let _ = fetch_trusted_setup(ctx, skde_path);
    let rpc_server = RpcServer::new(ctx.clone())
        .register_rpc_method::<GetTrustedSetup>()?
        .init(config.external_rpc_url.clone())
        .await
        .map_err(Error::from)?;

    info!("RPC server runs at {}", config.external_rpc_url);

    let handle = ctx.async_task().spawn_task(Box::pin(async move { rpc_server.stopped().await; }));

    Ok(vec![handle])
}


pub fn run_setup_trusted_setup<C: AppState>(ctx: &C, config: &Config) -> PathBuf {
    let skde_path = config.skde_path().join("skde_params.json");
    if skde_path.exists() {
        return skde_path;
    }
    let trusted_setup = ctx.secure_block().get_trusted_setup();
    let signature = ctx.sign(&trusted_setup).unwrap();
    let signed_params = SignedTrustedSetup { trusted_setup, signature };
    let serialized = serde_json::to_string_pretty(&signed_params).unwrap();
    fs::write(&skde_path, serialized).unwrap();
    info!("Successfully generated SKDE params at {:?}", skde_path);
    skde_path
}

pub fn fetch_trusted_setup<C: AppState>(ctx: &C, path: PathBuf) -> TrustedSetupFor<C> {
    info!("Fetching SKDE params from {:?}", path);
    match fs::read_to_string(&path) {
        Ok(data) => {
            match serde_json::from_str::<SignedTrustedSetup<C::Signature, TrustedSetupFor<C>>>(&data) {
                Ok(signed) => {
                    let _ = ctx.verify_signature(&signed.signature, &signed.trusted_setup, None).expect("Failed to verify SKDE params signature");
                    signed.trusted_setup
                },
                Err(e) => { panic!("Failed to parse SKDE param file: {}", e) }
            }
        }
        Err(e) => { panic!("SKDE params not set for authority node: {}", e) }
    }
}
