use std::path::PathBuf;
use radius_sdk::signature::ChainType;
pub use constants::*;
use serde::{Deserialize, Serialize};

mod constants {
    /// The default home path for storing configuration and data
    pub const DEFAULT_HOME_PATH: &str = ".radius";
    /// The directory name for storing database files
    pub const DATABASE_DIR_NAME: &str = "database";
    /// The file name for storing the signing key
    pub const SIGNING_KEY: &str = "signing_key";
    /// The default port number for external RPC communication
    pub const DEFAULT_EXTERNAL_RPC_URL: &str = "http://localhost:3000";
    /// The default port number for internal RPC communication
    pub const DEFAULT_INTERNAL_RPC_URL: &str = "http://localhost:4000";
    /// The default port number for cluster RPC communication
    pub const DEFAULT_CLUSTER_RPC_URL: &str = "http://localhost:5000";
    /// The default port number for authority RPC communication
    pub const DEFAULT_AUTHORITY_RPC_URL: &str = "http://localhost:7000";
    /// The default port number for solver RPC communication
    pub const DEFAULT_SOLVER_RPC_URL: &str = "http://localhost:6000";
    /// The default trusted address for the admin contract
    pub const DEFAULT_TRUSTED_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    /// The default blockchain type for signatures
    pub const DEFAULT_CHAIN_TYPE: &str = "ethereum";
    /// The default endpoint for the authentication service
    pub const DEFAULT_AUTH_SERVICE_URL: &str = "http://localhost:8545";
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    pub is_dev: bool,
    pub node_name: Option<String>,
    pub external_rpc_url: String,
    pub internal_rpc_url: String,
    pub cluster_rpc_url: String,
    pub role: Role,
    pub trusted_address: String,
    pub operator_service_url: String,
    pub chain_type: ChainType,
    pub private_key_path: PathBuf,
    pub db_path: PathBuf,
    pub trusted_setup_path: Option<PathBuf>,
}

impl NodeConfig {
    pub fn new(
        is_dev: bool,
        node_name: Option<String>,
        external_rpc_url: String, 
        internal_rpc_url: String, 
        cluster_rpc_url: String,
        role: Role,
        trusted_address: String,
        operator_service_url: String,
        chain_type: String,
        private_key_path: PathBuf,
        db_path: PathBuf,
        trusted_setup_path: Option<PathBuf>,
    ) -> Self {
        Self {
            is_dev,
            node_name,
            external_rpc_url,
            internal_rpc_url,
            cluster_rpc_url,
            role,
            trusted_address,
            operator_service_url,
            chain_type: chain_type.try_into().unwrap(),
            private_key_path,
            db_path,
            trusted_setup_path,
        }
    }

    pub fn trusted_setup_path(&self) -> PathBuf {
        self.trusted_setup_path.clone().expect("Trusted setup path not set")
    }

    pub fn log(&self) -> String {
        let mut log_lines = Vec::new();
        log_lines.push(format!("👤 Role: {}", self.role.to_string().to_uppercase()));
        if !self.role.is_authority() {
            log_lines.push("Is dev mode?: ".to_string() + &self.is_dev.to_string());
            log_lines.push(format!("🔍 Node Name: {}", self.node_name.clone().unwrap_or("N/A".to_string())));
            // Node role and basic info
            log_lines.push(format!("🔗 Chain Type: {:?}", self.chain_type));
                        
            // RPC endpoints
            log_lines.push(format!("🌐 External RPC: {}", self.external_rpc_url));
            log_lines.push(format!("🔒 Internal RPC: {}", self.internal_rpc_url));
            log_lines.push(format!("🔄 Cluster RPC: {}", self.cluster_rpc_url));

            // Security and configuration
            log_lines.push(format!("🔑 Trusted Address: {}", self.trusted_address));

            // Paths
            log_lines.push(format!("💾 DB opens at: {}", self.db_path.display()));
            if let Some(setup_path) = &self.trusted_setup_path {
                log_lines.push(format!("🔧 Trusted Setup Path: {}", setup_path.display()));
            }
        }
        
        // Auth service
        log_lines.push(format!("🔐 Operator Service URL: {}", self.operator_service_url));
        
        log_lines.join("\n")
    }
}

/// Roles in the DKG network
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
pub enum Role {
    /// Committee node that generates encryption keys and acts as a leader
    Committee,
    /// Solver node that computes decryption keys
    Solver,
    /// Verifier node that monitors the network for Byzantine behavior
    Verifier,
    /// Authority node that constructs the trusted setup
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

    pub fn is_authority(&self) -> bool {
        self == &Self::Authority
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
