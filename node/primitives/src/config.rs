use std::path::PathBuf;
use radius_sdk::signature::{Address, PrivateKeySigner, ChainType};

use crate::Role;

pub const DEFAULT_HOME_PATH: &str = ".radius";
pub const DATABASE_DIR_NAME: &str = "database";
pub const CONFIG_FILE_NAME: &str = "Config.toml";
pub const SIGNING_KEY: &str = "signing_key";

pub const DEFAULT_EXTERNAL_RPC_URL: &str = "http://127.0.0.1:3000";
pub const DEFAULT_INTERNAL_RPC_URL: &str = "http://127.0.0.1:4000";
pub const DEFAULT_CLUSTER_RPC_URL: &str = "http://127.0.0.1:5000";

pub const DEFAULT_RADIUS_FOUNDATION_ADDRESS: &str = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
pub const DEFAULT_CHAIN_TYPE: &str = "ethereum";

pub const DEFAULT_SESSION_CYCLE_MS: u64 = 500;

#[derive(Clone)]
pub struct Config {
    path: PathBuf,
    external_rpc_url: String,
    internal_rpc_url: String,
    cluster_rpc_url: String,
    solver_rpc_url: Option<String>,
    leader_cluster_rpc_url: Option<String>,
    leader_solver_rpc_url: Option<String>,
    solver_solver_rpc_url: Option<String>,
    authority_rpc_url: String,
    role: Role,
    signer: PrivateKeySigner,
    radius_foundation_address: Address,
    chain_type: ChainType,
    session_cycle: u64,
}

impl Config {

    pub fn new(
        path: PathBuf, 
        external_rpc_url: String, 
        internal_rpc_url: String, 
        cluster_rpc_url: String,
        solver_rpc_url: Option<String>,
        leader_cluster_rpc_url: Option<String>,
        leader_solver_rpc_url: Option<String>,
        solver_solver_rpc_url: Option<String>,
        authority_rpc_url: String,
        role: Role,
        signer: PrivateKeySigner,
        radius_foundation_address: Address,
        chain_type: ChainType,
        session_cycle: u64,
    ) -> Self {
        Self {
            path,
            external_rpc_url,
            internal_rpc_url,
            cluster_rpc_url,
            solver_rpc_url,
            leader_cluster_rpc_url,
            leader_solver_rpc_url,
            solver_solver_rpc_url,
            authority_rpc_url,
            role,
            signer,
            radius_foundation_address,
            chain_type,
            session_cycle,
        }
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

    pub fn session_cycle(&self) -> u64 {
        self.session_cycle
    }

    pub fn cluster_rpc_url(&self) -> &String {
        &self.cluster_rpc_url
    }

    pub fn solver_rpc_url(&self) -> &Option<String> {
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
        matches!(&self.role, Role::Leader)
    }

    pub fn is_committee(&self) -> bool {
        matches!(&self.role, Role::Committee)
    }

    pub fn is_solver(&self) -> bool {
        matches!(&self.role, Role::Solver)
    }

    pub fn is_verifier(&self) -> bool {
        matches!(&self.role, Role::Verifier)
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
