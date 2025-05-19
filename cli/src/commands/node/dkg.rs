use crate::Args;
use dkg_node_primitives::Role;
use dkg_node_primitives::config::{DEFAULT_TRUSTED_ADDRESS, DEFAULT_CHAIN_TYPE, DEFAULT_SESSION_CYCLE_MS};

#[derive(Debug, Args)]
pub struct DkgArgs {
    #[arg(long = "dkg.role", default_value_t = Role::Committee)]
    pub role: Role,
    #[arg(long = "dkg.trusted-address", default_value_t = DEFAULT_TRUSTED_ADDRESS.to_string())]
    pub trusted_address: String,
    #[arg(long = "dkg.chaintype", default_value_t = DEFAULT_CHAIN_TYPE.to_string())]
    pub chain_type: String,
    #[arg(long = "dkg.session-cycle", default_value_t = DEFAULT_SESSION_CYCLE_MS)]
    pub session_cycle: u64,
}

impl Default for DkgArgs {
    fn default() -> Self {
        Self {
            role: Role::Committee,
            trusted_address: DEFAULT_TRUSTED_ADDRESS.to_string(),
            chain_type: DEFAULT_CHAIN_TYPE.to_string(),
            session_cycle: DEFAULT_SESSION_CYCLE_MS,
        }
    }
}