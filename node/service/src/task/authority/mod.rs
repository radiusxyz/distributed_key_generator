use std::{fs, path::PathBuf};
use dkg_primitives::SignedSkdeParams;
use dkg_node_primitives::{DEFAULT_GENERATOR, DEFAULT_MAX_SEQUENCER_NUMBER, DEFAULT_TIME_PARAM_T};
use dkg_rpc::external::GetSkdeParams;
use skde::delay_encryption::setup;
use tracing::info;

use super::{SkdeParams, RpcServer, AppState, DkgAppState, Config, Error};

pub async fn run_node(ctx: &mut DkgAppState, config: Config) -> Result<(), Error> {
    let skde_path = config.skde_path.expect("SKDE path not set");
    run_setup_skde_params(ctx, skde_path.clone());
    let skde_params = fetch_skde_params(ctx, skde_path);
    ctx.with_skde_params(skde_params);
    
    let rpc_server = RpcServer::new(ctx.clone())
        .register_rpc_method::<GetSkdeParams>()?
        .init(config.external_rpc_url.clone())
        .await
        .map_err(Error::from)?;

    info!("RPC server runs at {}", config.external_rpc_url);

    ctx.spawn_task(Box::pin(async move { rpc_server.stopped().await; }));

    Ok(())
}


pub fn run_setup_skde_params<C: AppState>(ctx: &C, path: PathBuf) {
    let skde_path = path.join("skde_params.json");
    let params = setup(
        DEFAULT_TIME_PARAM_T,
        DEFAULT_GENERATOR.into(),
        DEFAULT_MAX_SEQUENCER_NUMBER.into(),
    );
    let signature = ctx.sign(&params).unwrap();
    let signed_params = SignedSkdeParams { params, signature };
    let serialized = serde_json::to_string_pretty(&signed_params).unwrap();
    fs::write(&skde_path, serialized).unwrap();
    info!("Successfully generated SKDE params at {:?}", skde_path);
}

pub fn fetch_skde_params<C: AppState>(ctx: &C, path: PathBuf) -> SkdeParams {
    match fs::read_to_string(&path) {
        Ok(data) => {
            match serde_json::from_str::<SignedSkdeParams<C::Signature>>(&data) {
                Ok(signed) => {
                    let _ = ctx.verify_signature(&signed.signature, &signed.params, None).expect("Failed to verify SKDE params signature");
                    signed.params
                },
                Err(e) => { panic!("Failed to parse SKDE param file: {}", e) }
            }
        }
        Err(e) => { panic!("SKDE params not set for authority node: {}", e) }
    }
}
