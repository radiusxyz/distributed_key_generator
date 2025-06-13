use crate::Args;
use dkg_node_primitives::config::{Role, DEFAULT_TRUSTED_ADDRESS, DEFAULT_CHAIN_TYPE, DEFAULT_SESSION_DURATION, DEFAULT_THRESHOLD, DEFAULT_AUTH_SERVICE_ENDPOINT, DEFAULT_ROUND_LOOK_AHEAD};

/// The arguments needed for DKG process
#[derive(Debug, Args)]
pub struct DkgArgs {
    #[arg(long = "dkg.role", default_value_t = Role::Committee)]
    pub role: Role,
    /// The address of the trusted address(e.g Admin contract address)
    #[arg(long = "dkg.trusted-address", default_value_t = DEFAULT_TRUSTED_ADDRESS.to_string())]
    pub trusted_address: String,
    /// The endpoint of the auth service(e.g blockchain rpc endpoint)
    #[arg(long = "dkg.auth-service-endpoint", default_value_t = DEFAULT_AUTH_SERVICE_ENDPOINT.to_string())]
    pub auth_service_endpoint: String,
    /// The type of the chain for signature type(e.g ethereum, solana)
    #[arg(long = "dkg.chaintype", default_value_t = DEFAULT_CHAIN_TYPE.to_string())]
    pub chain_type: String,
    /// The session cycle in milliseconds
    #[arg(long = "dkg.session-duration", default_value_t = DEFAULT_SESSION_DURATION)]
    pub session_duration: u64,
    /// The threshold of encryption key submission
    #[arg(long = "dkg.threshold", default_value_t = DEFAULT_THRESHOLD)]
    pub threshold: u16,
    /// The round look ahead for the DKG process
    #[arg(long = "dkg.round-look-ahead", default_value_t = DEFAULT_ROUND_LOOK_AHEAD)]
    pub round_look_ahead: u64,
}

impl Default for DkgArgs {
    fn default() -> Self {
        Self {
            role: Role::Committee,
            trusted_address: DEFAULT_TRUSTED_ADDRESS.to_string(),
            auth_service_endpoint: DEFAULT_AUTH_SERVICE_ENDPOINT.to_string(),
            chain_type: DEFAULT_CHAIN_TYPE.to_string(),
            session_duration: DEFAULT_SESSION_DURATION,
            threshold: DEFAULT_THRESHOLD,
            round_look_ahead: DEFAULT_ROUND_LOOK_AHEAD,
        }
    }
}