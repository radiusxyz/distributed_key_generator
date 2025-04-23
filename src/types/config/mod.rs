mod config_option;
mod config_path;

use std::{fs, path::PathBuf};

pub use config_option::*;
pub use config_path::*;
use radius_sdk::signature::{Address, ChainType, PrivateKeySigner};
use tracing::info;

pub const DEFAULT_HOME_PATH: &str = ".radius";
pub const DATABASE_DIR_NAME: &str = "database";
pub const CONFIG_FILE_NAME: &str = "Config.toml";
pub const SIGNING_KEY: &str = "signing_key";

const DEFAULT_EXTERNAL_RPC_URL: &str = "http://127.0.0.1:3000";
const DEFAULT_INTERNAL_RPC_URL: &str = "http://127.0.0.1:4000";
const DEFAULT_CLUSTER_RPC_URL: &str = "http://127.0.0.1:5000";

const DEFAULT_RADIUS_FOUNDATION_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
const DEFAULT_CHAIN_TYPE: &str = "ethereum";

const DEFAULT_PARTIAL_KEY_GENERATION_CYCLE_MS: u64 = 500;
const DEFAULT_PARTIAL_KEY_AGGREGATION_CYCLE_MS: u64 = 500;

#[derive(Clone)]
pub struct Config {
    path: PathBuf,

    external_rpc_url: String,
    internal_rpc_url: String,
    cluster_rpc_url: String,
    solver_rpc_url: String,
    leader_cluster_rpc_url: Option<String>,
    leader_solver_rpc_url: Option<String>,
    solver_solver_rpc_url: Option<String>,
    authority_rpc_url: String,
    role: Role,

    signer: PrivateKeySigner,

    radius_foundation_address: Address,
    chain_type: ChainType,

    partial_key_generation_cycle_ms: u64,
    partial_key_aggregation_cycle_ms: u64,
}

impl Config {
    pub fn load(config_option: &mut ConfigOption) -> Result<Self, ConfigError> {
        let config_path = match config_option.path.as_mut() {
            Some(config_path) => config_path.clone(),
            None => {
                let config_path: PathBuf = ConfigPath::default().as_ref().into();
                config_option.path = Some(config_path.clone());
                config_path
            }
        };

        // Read config file
        let config_file_path = config_path.join(CONFIG_FILE_NAME);

        // Try to read config file, if it doesn't exist or can't be read, use default values
        let config_file: ConfigOption = if config_file_path.exists() {
            match fs::read_to_string(&config_file_path) {
                Ok(config_string) => match toml::from_str(&config_string) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        tracing::warn!("Failed to parse config file: {}, using default values", e);
                        ConfigOption::default()
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config file: {}, using default values", e);
                    ConfigOption::default()
                }
            }
        } else {
            tracing::warn!(
                "Config file not found at {:?}, using default values",
                config_file_path
            );
            ConfigOption::default()
        };

        // Merge configs from CLI input
        let merged_config_option = config_file.merge(config_option);
        info!("chain_type: {:?}", merged_config_option);

        let chain_type = merged_config_option.chain_type.unwrap().try_into().unwrap();

        // Read signing key
        let signing_key_path = config_path.join(SIGNING_KEY);

        let signer = if signing_key_path.exists() {
            match fs::read_to_string(&signing_key_path) {
                Ok(key_string) => {
                    let clean_key = key_string.trim().replace("\n", "").replace("\r", "");
                    match PrivateKeySigner::from_str(chain_type, &clean_key) {
                        Ok(signer) => signer,
                        Err(err) => {
                            tracing::warn!(
                                "Invalid signing key in file: {}, using default key",
                                err
                            );
                            tracing::warn!("Key string was: '{}'", clean_key);
                            let default_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
                            PrivateKeySigner::from_str(chain_type, default_key).unwrap()
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "Failed to read signing key file: {}, using default key",
                        err
                    );
                    let default_key =
                        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
                    PrivateKeySigner::from_str(chain_type, default_key).unwrap()
                }
            }
        } else {
            tracing::warn!(
                "Signing key file not found at {:?}, using default key",
                signing_key_path
            );
            // Create directory if it doesn't exist
            if let Some(parent) = signing_key_path.parent() {
                if !parent.exists() {
                    let _ = fs::create_dir_all(parent);
                }
            }
            // Write default key to file
            let default_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
            let _ = fs::write(&signing_key_path, default_key);
            PrivateKeySigner::from_str(chain_type, default_key).unwrap()
        };

        // Parse role if provided
        let role = if let Some(role_str) = &merged_config_option.role {
            match role_str.parse::<Role>() {
                Ok(role) => role,
                Err(e) => {
                    tracing::warn!("Invalid role: {}, ignoring role setting", e);
                    Role::Committee
                }
            }
        } else {
            Role::Committee
        };

        Ok(Config {
            path: config_path,
            external_rpc_url: merged_config_option.external_rpc_url.unwrap(),
            internal_rpc_url: merged_config_option.internal_rpc_url.unwrap(),
            cluster_rpc_url: merged_config_option.cluster_rpc_url.unwrap(),
            solver_rpc_url: merged_config_option.solver_rpc_url.unwrap(),
            leader_cluster_rpc_url: merged_config_option.leader_cluster_rpc_url.clone(),
            leader_solver_rpc_url: merged_config_option.leader_solver_rpc_url.clone(),
            solver_solver_rpc_url: merged_config_option.solver_solver_rpc_url.clone(),
            authority_rpc_url: merged_config_option.authority_rpc_url.unwrap(),
            role,
            signer,
            radius_foundation_address: Address::from_str(
                chain_type,
                &merged_config_option.radius_foundation_address.unwrap(),
            )
            .unwrap(),
            chain_type,

            partial_key_generation_cycle_ms: merged_config_option
                .partial_key_generation_cycle_ms
                .unwrap(),
            partial_key_aggregation_cycle_ms: merged_config_option
                .partial_key_aggregation_cycle_ms
                .unwrap(),
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn database_path(&self) -> PathBuf {
        self.path.join(DATABASE_DIR_NAME)
    }

    pub fn signer(&self) -> &PrivateKeySigner {
        &self.signer
    }

    pub fn radius_foundation_address(&self) -> &Address {
        &self.radius_foundation_address
    }

    pub fn chain_type(&self) -> &ChainType {
        &self.chain_type
    }

    pub fn address(&self) -> &Address {
        self.signer().address()
    }

    pub fn external_rpc_url(&self) -> &String {
        &self.external_rpc_url
    }

    pub fn internal_rpc_url(&self) -> &String {
        &self.internal_rpc_url
    }

    pub fn partial_key_generation_cycle_ms(&self) -> u64 {
        self.partial_key_generation_cycle_ms
    }

    pub fn partial_key_aggregation_cycle_ms(&self) -> u64 {
        self.partial_key_aggregation_cycle_ms
    }

    pub fn cluster_rpc_url(&self) -> &String {
        &self.cluster_rpc_url
    }

    pub fn solver_rpc_url(&self) -> &str {
        &self.solver_rpc_url
    }

    pub fn leader_cluster_rpc_url(&self) -> &Option<String> {
        &self.leader_cluster_rpc_url
    }

    pub fn leader_solver_rpc_url(&self) -> &Option<String> {
        &self.leader_solver_rpc_url
    }

    pub fn solver_solver_rpc_url(&self) -> &Option<String> {
        &self.solver_solver_rpc_url
    }

    pub fn authority_rpc_url(&self) -> &String {
        &self.authority_rpc_url
    }

    pub fn role(&self) -> &Role {
        &self.role
    }

    pub fn is_authority(&self) -> bool {
        matches!(self.role, Role::Authority)
    }

    pub fn is_leader(&self) -> bool {
        match &self.role {
            Role::Leader => true,
            _ => self.leader_cluster_rpc_url.is_none(), // For backward compatibility
        }
    }

    pub fn is_committee(&self) -> bool {
        match &self.role {
            Role::Committee => true,
            _ => true, // Default behavior is committee
        }
    }

    pub fn is_solver(&self) -> bool {
        match &self.role {
            Role::Solver => true,
            _ => false,
        }
    }

    pub fn is_verifier(&self) -> bool {
        match &self.role {
            Role::Verifier => true,
            _ => false,
        }
    }

    pub fn external_port(&self) -> Result<String, ConfigError> {
        Ok(self
            .external_rpc_url()
            .split(':')
            .last()
            .ok_or(ConfigError::InvalidExternalPort)?
            .to_string())
    }

    pub fn cluster_port(&self) -> Result<String, ConfigError> {
        Ok(self
            .cluster_rpc_url()
            .split(':')
            .last()
            .ok_or(ConfigError::InvalidClusterPort)?
            .to_string())
    }
    pub fn authority_port(&self) -> Result<String, ConfigError> {
        Ok(self
            .authority_rpc_url()
            .split(':')
            .last()
            .ok_or(ConfigError::InvalidClusterPort)?
            .to_string())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Load(std::io::Error),
    Parse(toml::de::Error),
    RemoveConfigDirectory(std::io::Error),
    CreateConfigDirectory(std::io::Error),
    CreateConfigFile(std::io::Error),
    CreatePrivateKeyFile(std::io::Error),

    InvalidExternalPort,
    InvalidClusterPort,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ConfigError {}
