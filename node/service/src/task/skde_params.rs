use std::{fs, str::FromStr};

use dkg_primitives::SignedSkgdeParams;
use dkg_rpc::{
    authority::{GetAuthorizedSkdeParams, GetAuthorizedSkdeParamsResponse},
    common::{GetSkdeParams, GetSkdeParamsResponse},
};
use dkg_utils::{log::log_prefix, signature::verify_signature};
use radius_sdk::json_rpc::{
    client::{Id, RpcClient, RpcClientError},
    server::RpcParameter,
};
use skde::{delay_encryption::SkdeParams, BigUint};
use tracing::{error, info, warn};

use dkg_node_primitives::{Config, Role};
use crate::task::authority_setup::SignedSkdeParams;

async fn fetch_skde_params(config: &Config) -> Option<SkdeParams> {
    let prefix = log_prefix(config);
    match config.role() {
        Role::Authority => {
            let skde_path = config.path().join("skde_params.json");

            match fs::read_to_string(&skde_path) {
                Ok(data) => {
                    info!(
                        "{} Successfully read SKDE param file, length: {}",
                        prefix,
                        data.len()
                    );

                    match serde_json::from_str::<SignedSkdeParams>(&data) {
                        Ok(signed) => match verify_signature(&signed.signature, &signed.params) {
                            Ok(_signer_address) => {
                                info!("{} Successfully verified SKDE params signature", prefix);
                                Some(signed.params)
                            }
                            Err(e) => {
                                warn!("{} Failed to verify SKDE params signature: {}", prefix, e);
                                None
                            }
                        },
                        Err(e) => {
                            error!("Failed to parse SKDE param file: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "{} Failed to read SKDE param file at {:?}: {}",
                        prefix, skde_path, e
                    );
                    warn!(
                        "{} Must run `setup-skde-params` to initialize SKDE parameters.",
                        prefix
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
                    warn!("{} Failed to create RPC client: {}", prefix, err);
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
                Ok(response) => {
                    let signed = response.signed_skde_params;

                    match verify_signature(&signed.signature, &signed.params) {
                        Ok(_signer_address) => {
                            info!(
                                "{} Successfully verified SKDE params signature from authority",
                                prefix
                            );
                            Some(signed.params) // 검증 성공 시 params만 넘김
                        }
                        Err(e) => {
                            warn!("{} Failed to verify SKDE params signature: {}", prefix, e);
                            None
                        }
                    }
                }
                Err(err) => {
                    warn!(
                        "{} Failed to fetch SkdeParams from authority: {}",
                        prefix, err
                    );
                    None
                }
            }
        }

        Role::Committee => {
            if let Some(leader_url) = config.leader_cluster_rpc_url() {
                let client = match RpcClient::new() {
                    Ok(c) => c,
                    Err(err) => {
                        warn!("{} Failed to create RPC client: {}", prefix, err);
                        return None;
                    }
                };

                let result: Result<GetSkdeParamsResponse, RpcClientError> = client
                    .request(
                        leader_url,
                        GetSkdeParams::method(),
                        &GetSkdeParams,
                        Id::Null,
                    )
                    .await;

                match result {
                    Ok(response) => {
                        let signed = response.signed_skde_params;

                        match verify_signature(&signed.signature, &signed.params) {
                            Ok(_signer_address) => {
                                info!(
                                    "{} Successfully verified SKDE params signature from leader",
                                    prefix
                                );
                                Some(signed.params)
                            }
                            Err(e) => {
                                warn!("{} Failed to verify SKDE params signature: {}", prefix, e);
                                None
                            }
                        }
                    }
                    Err(err) => {
                        warn!("{} Failed to fetch SkdeParams from leader: {}", prefix, err);
                        None
                    }
                }
            } else {
                warn!("{} Missing leader_cluster_rpc_url in config", prefix);
                None
            }
        }

        Role::Solver => {
            if let Some(leader_url) = config.leader_solver_rpc_url() {
                let client = match RpcClient::new() {
                    Ok(c) => c,
                    Err(err) => {
                        warn!("{} Failed to create RPC client: {}", prefix, err);
                        return None;
                    }
                };

                let result: Result<GetSkdeParamsResponse, RpcClientError> = client
                    .request(
                        leader_url,
                        GetSkdeParams::method(),
                        &GetSkdeParams,
                        Id::Null,
                    )
                    .await;

                match result {
                    Ok(response) => {
                        let signed = response.signed_skde_params;

                        match verify_signature(&signed.signature, &signed.params) {
                            Ok(_signer_address) => {
                                info!(
                                    "{} Successfully verified SKDE params signature from leader",
                                    prefix
                                );
                                Some(signed.params)
                            }
                            Err(e) => {
                                warn!("{} Failed to verify SKDE params signature: {}", prefix, e);
                                None
                            }
                        }
                    }
                    Err(err) => {
                        warn!("{} Failed to fetch SkdeParams from leader: {}", prefix, err);
                        None
                    }
                }
            } else {
                warn!("{} Missing leader_solver_rpc_url in config", prefix);
                None
            }
        }

        _ => {
            warn!("{} Unsupported role for SKDE param retrieval", prefix,);
            None
        }
    }
}

/// Keep retrying until SKDE params are fetched successfully.
/// Panics if something unexpected goes wrong.
/// TODO: Appropriate error handling and retry limits
pub async fn fetch_skde_params_with_retry(config: &Config) -> SkdeParams {
    let prefix = log_prefix(config);
    loop {
        if let Some(params) = fetch_skde_params(config).await {
            info!("{} Successfully fetched SKDE params", prefix);
            return params;
        }

        warn!("{} Failed to fetch SKDE params, retrying in 1s...", prefix,);
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
