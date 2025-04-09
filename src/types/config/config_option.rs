use std::path::PathBuf;

use clap::Parser;
use serde::{Deserialize, Serialize};

use super::{
    config_path::ConfigPath, DEFAULT_CHAIN_TYPE, DEFAULT_CLUSTER_RPC_URL, DEFAULT_EXTERNAL_RPC_URL,
    DEFAULT_INTERNAL_RPC_URL, DEFAULT_PARTIAL_KEY_AGGREGATION_CYCLE,
    DEFAULT_PARTIAL_KEY_GENERATION_CYCLE, DEFAULT_RADIUS_FOUNDATION_ADDRESS,
};

/// Node roles in the DKG network
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Role {
    /// Leader node responsible for collecting partial keys and coordinating
    Leader,
    /// Committee node that generates partial keys
    Committee,
    /// Solver node that computes decryption keys
    Solver,
    /// Verifier node that monitors the network for Byzantine behavior
    Verifier,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Leader => write!(f, "leader"),
            Role::Committee => write!(f, "committee"),
            Role::Solver => write!(f, "solver"),
            Role::Verifier => write!(f, "verifier"),
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
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

#[derive(Debug, Deserialize, Parser, Serialize)]
pub struct ConfigOption {
    #[doc = "Set the configuration file path to load from"]
    #[clap(long = "path")]
    pub path: Option<PathBuf>,

    #[doc = "Set the external rpc url"]
    #[clap(long = "external-rpc-url")]
    pub external_rpc_url: Option<String>,

    #[doc = "Set the internal rpc url"]
    #[clap(long = "internal-rpc-url")]
    pub internal_rpc_url: Option<String>,

    #[doc = "Set the cluster rpc url"]
    #[clap(long = "cluster-rpc-url")]
    pub cluster_rpc_url: Option<String>,

    #[doc = "Set the leader cluster rpc url (previously seed-cluster-rpc-url)"]
    #[clap(long = "leader-cluster-rpc-url")]
    pub leader_cluster_rpc_url: Option<String>,

    #[doc = "Set the node role (leader, committee, solver, verifier)"]
    #[clap(long = "role")]
    pub role: Option<String>,

    #[doc = "Set the radius foundation address"]
    #[clap(long = "radius-foundation-address")]
    pub radius_foundation_address: Option<String>,

    #[doc = "Set the chain type (for verifying signature for foundation address)"]
    #[clap(long = "chain-type")]
    pub chain_type: Option<String>,

    #[doc = "Set partial key generation cycle"]
    #[clap(long = "partial-key-generation-cycle")]
    pub partial_key_generation_cycle: Option<u64>,

    #[doc = "Set partial key aggregation cycle"]
    #[clap(long = "partial-key-aggregation-cycle")]
    pub partial_key_aggregation_cycle: Option<u64>,
}

impl Default for ConfigOption {
    fn default() -> Self {
        Self {
            path: Some(ConfigPath::default().as_ref().into()),

            external_rpc_url: Some(DEFAULT_EXTERNAL_RPC_URL.into()),
            internal_rpc_url: Some(DEFAULT_INTERNAL_RPC_URL.into()),
            cluster_rpc_url: Some(DEFAULT_CLUSTER_RPC_URL.into()),
            leader_cluster_rpc_url: None,
            role: None,
            radius_foundation_address: Some(DEFAULT_RADIUS_FOUNDATION_ADDRESS.into()),
            chain_type: Some(DEFAULT_CHAIN_TYPE.into()),
            partial_key_generation_cycle: Some(DEFAULT_PARTIAL_KEY_GENERATION_CYCLE),
            partial_key_aggregation_cycle: Some(DEFAULT_PARTIAL_KEY_AGGREGATION_CYCLE),
        }
    }
}

impl ConfigOption {
    pub fn get_toml_string(&self) -> String {
        let mut toml_string = String::new();

        set_toml_comment(&mut toml_string, "Set external rpc url");
        set_toml_name_value(&mut toml_string, "external_rpc_url", &self.external_rpc_url);

        set_toml_comment(&mut toml_string, "Set internal rpc url");
        set_toml_name_value(&mut toml_string, "internal_rpc_url", &self.internal_rpc_url);

        set_toml_comment(&mut toml_string, "Set cluster rpc url");
        set_toml_name_value(&mut toml_string, "cluster_rpc_url", &self.cluster_rpc_url);

        set_toml_comment(&mut toml_string, "Set leader cluster rpc url");
        set_toml_name_value(
            &mut toml_string,
            "leader_cluster_rpc_url",
            &self.leader_cluster_rpc_url,
        );

        set_toml_comment(
            &mut toml_string,
            "Set node role (leader, committee, solver, verifier)",
        );
        set_toml_name_value(&mut toml_string, "role", &self.role);

        set_toml_comment(&mut toml_string, "Set the radius foundation address");
        set_toml_name_value(
            &mut toml_string,
            "radius_foundation_address",
            &self.radius_foundation_address,
        );

        set_toml_comment(
            &mut toml_string,
            "Set the chain type (for verifying signature for foundation address)",
        );
        set_toml_name_value(&mut toml_string, "chain_type", &self.chain_type);

        set_toml_comment(&mut toml_string, "Set partial key generation cycle");
        set_toml_name_value(
            &mut toml_string,
            "partial_key_generation_cycle",
            &self.partial_key_generation_cycle,
        );

        set_toml_comment(&mut toml_string, "Set partial key aggregation cycle");
        set_toml_name_value(
            &mut toml_string,
            "partial_key_aggregation_cycle",
            &self.partial_key_aggregation_cycle,
        );

        toml_string
    }

    pub fn merge(mut self, other: &ConfigOption) -> Self {
        if other.path.is_some() {
            self.path.clone_from(&other.path);
        }

        if other.external_rpc_url.is_some() {
            self.external_rpc_url.clone_from(&other.external_rpc_url);
        }

        if other.internal_rpc_url.is_some() {
            self.internal_rpc_url.clone_from(&other.internal_rpc_url);
        }

        if other.cluster_rpc_url.is_some() {
            self.cluster_rpc_url.clone_from(&other.cluster_rpc_url);
        }

        if other.leader_cluster_rpc_url.is_some() {
            self.leader_cluster_rpc_url
                .clone_from(&other.leader_cluster_rpc_url);
        }

        if other.role.is_some() {
            self.role.clone_from(&other.role);
        }

        if other.radius_foundation_address.is_some() {
            self.radius_foundation_address
                .clone_from(&other.radius_foundation_address);
        }

        if other.chain_type.is_some() {
            self.chain_type.clone_from(&other.chain_type);
        }

        if other.partial_key_generation_cycle.is_some() {
            self.partial_key_generation_cycle
                .clone_from(&other.partial_key_generation_cycle);
        }

        if other.partial_key_aggregation_cycle.is_some() {
            self.partial_key_aggregation_cycle
                .clone_from(&other.partial_key_aggregation_cycle);
        }

        self
    }
}

fn set_toml_comment(toml_string: &mut String, comment: &'static str) {
    let comment = format!("# {}\n", comment);

    toml_string.push_str(&comment);
}

fn set_toml_name_value<T>(toml_string: &mut String, name: &'static str, value: &Option<T>)
where
    T: std::fmt::Debug,
{
    let name_value = match value {
        Some(value) => format!("{} = {:?}\n\n", name, value),
        None => format!("# {} = {:?}\n\n", name, value),
    };

    toml_string.push_str(&name_value);
}
