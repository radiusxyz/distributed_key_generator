use std::path::PathBuf;
use dkg_primitives::RuntimeResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TomlConfig {
    pub is_dev: Option<bool>,
    pub node_name: Option<String>,
    pub external_rpc: Option<String>,
    pub internal_rpc: Option<String>,
    pub cluster_rpc: Option<String>,
    pub role: Option<String>,
    pub trusted_address: Option<String>,
    pub blockchain_http_rpc_url: Option<String>,
    pub blockchain_ws_rpc_url: Option<String>,
    pub chain_type: Option<String>,
    pub private_key_path: Option<PathBuf>,
    pub db_path: Option<PathBuf>,
    pub trusted_setup_path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExternalRpcConfig {
    pub authority_rpc_url: Option<String>,
    pub solver_rpc_url: Option<String>,
}

pub fn load_config(config_path: Option<PathBuf>) -> RuntimeResult<Option<TomlConfig>> {
    let path = match config_path {
        Some(path) => {
            if !path.exists() {
                return Ok(None);
            }
            path
        }, 
        None => {
            let default_path = PathBuf::from("config.toml");
            if !default_path.exists() {
                return Ok(None);
            }
            default_path
        }
    };
    let content = std::fs::read_to_string(path.to_str().unwrap()).expect("Failed to read config file");
    let config: TomlConfig = toml::from_str(&content).expect("Failed to parse config file");
    Ok(Some(config))
}