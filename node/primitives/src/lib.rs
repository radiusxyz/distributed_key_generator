mod config;
mod skde;
mod state;

pub use config::*;
pub use skde::*;

pub const DEFAULT_HOME_PATH: &str = ".radius";
pub const DATABASE_DIR_NAME: &str = "database";
pub const CONFIG_FILE_NAME: &str = "Config.toml";
pub const SIGNING_KEY: &str = "signing_key";

use serde::{Deserialize, Serialize};

/// Node roles in the DKG network
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum Role {
    /// Leader node responsible for collecting partial keys and coordinating
    Leader,
    /// Committee node that generates partial keys
    Committee,
    /// Solver node that computes decryption keys
    Solver,
    /// Verifier node that monitors the network for Byzantine behavior
    Verifier,
    /// Authority node that conducts the secure skde parameter setup
    Authority,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Leader => write!(f, "leader"),
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
            "leader" => Ok(Role::Leader),
            "committee" => Ok(Role::Committee),
            "solver" => Ok(Role::Solver),
            "verifier" => Ok(Role::Verifier),
            "authority" => Ok(Role::Authority),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}
