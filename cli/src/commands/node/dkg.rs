use crate::Args;
use dkg_node_primitives::config::Role;

/// The arguments needed for DKG process
#[derive(Debug, Args)]
pub struct DkgArgs {
    #[arg(long = "dkg.role")]
    pub role: Option<Role>,
    /// The address of the trusted address(e.g Admin contract address)
    #[arg(long = "dkg.trusted-address")]
    pub trusted_address: Option<String>,
    /// The endpoint of the auth service(e.g blockchain rpc endpoint)
    #[arg(long = "dkg.blockchain-http-rpc-url")]
    pub blockchain_http_rpc_url: Option<String>,
    #[arg(long = "dkg.blockchain-ws-rpc-url")]
    pub blockchain_ws_rpc_url: Option<String>,
    /// The type of the chain for signature type(e.g ethereum, solana)
    #[arg(long = "dkg.chaintype")]
    pub chain_type: Option<String>,
    /// The session cycle in milliseconds
    #[arg(long = "dkg.session-duration")]
    pub session_duration: Option<u64>,
    /// The duration of collecting key in milliseconds
    #[arg(long = "dkg.collecting-duration")]
    pub collecting_duration: Option<u64>,
    /// The threshold of encryption key submission
    #[arg(long = "dkg.threshold")]
    pub threshold: Option<u16>,
    /// The round look ahead for the DKG process
    #[arg(long = "dkg.round-look-ahead")]
    pub round_look_ahead: Option<u64>,
}