use std::{fs, str::FromStr};

use radius_sdk::json_rpc::{
    client::{Id, RpcClient, RpcClientError},
    server::RpcParameter,
};
use skde::{delay_encryption::SkdeParams, BigUint};
use tracing::info;

use super::{Config, Role};
use crate::rpc::{
    authority::{GetAuthorizedSkdeParams, GetAuthorizedSkdeParamsResponse},
    cluster::{GetSkdeParams, GetSkdeParamsResponse},
};

async fn fetch_skde_params(config: &Config) -> Option<SkdeParams> {
    match config.role() {
        Role::Authority => {
            let skde_path = config.path().join("skde_params.json");

            match fs::read_to_string(&skde_path) {
                Ok(data) => {
                    info!("Successfully read SKDE param file, length: {}", data.len());
                    match serde_json::from_str::<SkdeParams>(&data) {
                        Ok(parsed) => Some(parsed),
                        Err(e) => {
                            tracing::error!("Failed to parse SKDE param file: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read SKDE param file at {:?}: {}", skde_path, e);
                    tracing::warn!(
                        "Authority node must run `setup-skde-params` to initialize SKDE parameters."
                    );
                    None
                }
            }
        }

        Role::Leader => {
            let authority_url = config.authority_rpc_url(); // &String

            let client = match RpcClient::new() {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("Failed to create RPC client: {}", err);
                    return None;
                }
            };

            let result: Result<GetAuthorizedSkdeParamsResponse, RpcClientError> = client
                .request(
                    authority_url,
                    GetAuthorizedSkdeParams::method(),
                    &GetAuthorizedSkdeParams,
                    Id::Null,
                )
                .await;

            match result {
                Ok(response) => Some(response.into_skde_params()),
                Err(err) => {
                    tracing::warn!("Failed to fetch SkdeParams from authority: {}", err);
                    None
                }
            }
        }

        _ => {
            if let Some(leader_url) = config.leader_cluster_rpc_url() {
                let client = match RpcClient::new() {
                    Ok(c) => c,
                    Err(err) => {
                        tracing::warn!("Failed to create RPC client: {}", err);
                        return None;
                    }
                };

                let response: GetSkdeParamsResponse = match client
                    .request(
                        leader_url,
                        GetSkdeParams::method(),
                        &GetSkdeParams,
                        Id::Null,
                    )
                    .await
                {
                    Ok(res) => res,
                    Err(err) => {
                        tracing::warn!("Failed to fetch SkdeParams from leader: {}", err);
                        return None;
                    }
                };

                Some(response.into_skde_params())
            } else {
                tracing::warn!("Missing leader_cluster_rpc_url in config");
                None
            }
        }
    }
}

/// Keep retrying until SKDE params are fetched successfully.
/// Panics if something unexpected goes wrong.
/// TODO: Appropriate error handling and retry limits
pub async fn fetch_skde_params_with_retry(config: &Config) -> SkdeParams {
    loop {
        if let Some(params) = fetch_skde_params(config).await {
            info!("Successfully fetched SKDE params");
            return params;
        }

        tracing::warn!("Failed to fetch SKDE params, retrying in 1s...");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Only for authority node
pub fn default_skde_params() -> SkdeParams {
    const MOD_N: &str = "26737688233630987849749538623559587294088037102809480632570023773459222152686633609232230584184543857897813615355225270819491245893096628373370101798393754657209853664433779631579690734503677773804892912774381357280025811519740953667880409246987453978226997595139808445552217486225687511164958368488319372068289768937729234964502681229612929764203977349037219047813560623373035187038018937232123821089208711930458219009895581132844064176371047461419609098259825422421077554570457718558971463292559934623518074946858187287041522976374186587813034651849410990884606427758413847140243755163116582922090226726575253150079";
    const GENERATOR: &str = "4";
    const TIME_PARAM_T: u32 = 2;
    const MAX_SEQUENCER_NUMBER: u32 = 2;

    let n = BigUint::from_str(MOD_N).expect("Invalid MOD_N");
    let g = BigUint::from_str(GENERATOR).expect("Invalid GENERATOR");
    let max = BigUint::from(MAX_SEQUENCER_NUMBER);
    let t = 2_u32.pow(TIME_PARAM_T);

    let mut h = g.clone();
    (0..t).for_each(|_| {
        h = (&h * &h) % &n;
    });

    SkdeParams {
        t,
        n: n.to_str_radix(10),
        g: g.to_str_radix(10),
        h: h.to_str_radix(10),
        max_sequencer_number: max.to_str_radix(10),
    }
}
