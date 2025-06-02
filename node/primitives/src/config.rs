use std::path::PathBuf;
use radius_sdk::signature::ChainType;
pub use constants::*;
use serde::{Deserialize, Serialize};

mod constants {
    pub const DEFAULT_HOME_PATH: &str = ".radius";
    pub const DATABASE_DIR_NAME: &str = "database";
    pub const SIGNING_KEY: &str = "signing_key";
    pub const DEFAULT_EXTERNAL_RPC_PORT: u16 = 3000;
    pub const DEFAULT_INTERNAL_RPC_PORT: u16 = 4000;
    pub const DEFAULT_CLUSTER_RPC_PORT: u16 = 5000;
    pub const DEFAULT_LEADER_RPC_PORT: u16 = 6000;
    pub const DEFAULT_AUTHORITY_RPC_PORT: u16 = 7000;
    pub const DEFAULT_TRUSTED_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    pub const DEFAULT_SESSION_CYCLE_MS: u64 = 2000; // 2s
    pub const DEFAULT_CHAIN_TYPE: &str = "ethereum";
    pub const DEFAULT_THRESHOLD: u16 = 1;
    pub const DEFAULT_AUTH_SERVICE_ENDPOINT: &str = "http://localhost:8545";
}

#[derive(Debug, Clone)]
pub struct Config {
    pub external_rpc_url: String,
    pub internal_rpc_url: String,
    pub cluster_rpc_url: String,
    pub maybe_authority_rpc_url: Option<String>,
    pub maybe_solver_rpc_url: Option<String>,
    pub role: Role,
    pub trusted_address: String,
    pub auth_service_endpoint: String,
    pub chain_type: ChainType,
    pub session_cycle: u64,
    pub private_key_path: PathBuf,
    pub db_path: PathBuf,
    pub trusted_setup_path: Option<PathBuf>,
    pub threshold: u16,
}

impl Config {
    pub fn new(
        external_rpc_url: String, 
        internal_rpc_url: String, 
        cluster_rpc_url: String,
        maybe_authority_rpc_url: Option<String>,
        maybe_solver_rpc_url: Option<String>,
        role: Role,
        trusted_address: String,
        auth_service_endpoint: String,
        chain_type: ChainType,
        session_cycle: u64,
        private_key_path: PathBuf,
        db_path: PathBuf,
        trusted_setup_path: Option<PathBuf>,
        threshold: u16,
    ) -> Self {
        Self {
            external_rpc_url,
            internal_rpc_url,
            cluster_rpc_url,
            maybe_authority_rpc_url,
            maybe_solver_rpc_url,
            role,
            trusted_address,
            auth_service_endpoint,
            chain_type,
            session_cycle,
            private_key_path,
            db_path,
            trusted_setup_path,
            threshold,
        }
    }

    pub fn trusted_setup_path(&self) -> PathBuf {
        self.trusted_setup_path.clone().expect("Trusted setup path not set")
    }

    pub fn session_cycle(&self) -> u64 {
        self.session_cycle
    }

    pub fn validate(&self) -> bool {
        match self.role {
            _ => true
        }
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
    InvalidConfig,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ConfigError {}

/// Roles in the DKG network
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Role {
    /// Committee node that generates encryption keys
    Committee,
    /// Solver node that computes decryption keys
    Solver,
    /// Verifier node that monitors the network for Byzantine behavior
    Verifier,
    /// Authority node that conducts the secure trusted setup
    Authority,
}

impl Role {
    /// Iterate over all active roles in the network
    pub fn iter_roles() -> impl Iterator<Item = Self> {
        vec![
            Self::Committee,
            Self::Solver,
            #[cfg(feature = "verifier")]
            Self::Verifier,
        ].into_iter()
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Committee => write!(f, "committee"),
            Role::Solver => write!(f, "solver"),
            Role::Verifier => write!(f, "verifier"),
            Role::Authority => write!(f, "authority"),
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "committee" => Ok(Role::Committee),
            "solver" => Ok(Role::Solver),
            "verifier" => Ok(Role::Verifier),
            "authority" => Ok(Role::Authority),
            _ => Err(format!("Unknown role. Might choose either: leader, committee, solver, verifier, authority")),
        }
    }
}
