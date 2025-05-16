use std::{env, fs, path::PathBuf};
use clap::Parser;
use dkg_node_primitives::{
    CONFIG_FILE_NAME, DEFAULT_CHAIN_TYPE, DEFAULT_CLUSTER_RPC_URL, DEFAULT_EXTERNAL_RPC_URL,
    DEFAULT_INTERNAL_RPC_URL, DEFAULT_RADIUS_FOUNDATION_ADDRESS, DEFAULT_SESSION_CYCLE_MS,
    ConfigError, DEFAULT_HOME_PATH
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser, Serialize, Deserialize)]
pub struct ConfigPath {
    #[doc = "Set the key generator configuration path"]
    #[clap(long = "path", default_value_t = Self::default().to_string())]
    path: String,
}

impl std::fmt::Display for ConfigPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl AsRef<Path> for ConfigPath {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Default for ConfigPath {
    fn default() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let path_arg = args.iter().enumerate().find_map(|(i, arg)| {
            if arg == "--path" && i + 1 < args.len() {
                Some(args[i + 1].clone())
            } else if arg.starts_with("--path=") {
                Some(arg.trim_start_matches("--path=").to_string())
            } else {
                None
            }
        });

        let path = match path_arg {
            Some(p) => p,
            None => match std::env::var("RADIUS_NODE_PATH") {
                Ok(env_path) => env_path,
                Err(_) => {
                    let current_dir = std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .to_string_lossy()
                        .to_string();

                    let node1_path = PathBuf::from(&current_dir).join("data").join("node1");

                    if node1_path.exists() && node1_path.join(CONFIG_FILE_NAME).exists() {
                        node1_path.to_string_lossy().to_string()
                    } else {
                        PathBuf::from(env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                            .join(DEFAULT_HOME_PATH)
                            .to_string_lossy()
                            .to_string()
                    }
                }
            },
        };

        Self { path }
    }
}

impl ConfigPath {
    pub fn init(&self) -> Result<(), ConfigError> {
        if !self.as_ref().exists() {
            fs::create_dir_all(self).map_err(ConfigError::CreateConfigDirectory)?;
        }

        let config_file_path = self.as_ref().join(CONFIG_FILE_NAME);
        if !config_file_path.exists() {
            let config_toml_string = ConfigOption::default().get_toml_string();
            fs::write(config_file_path, config_toml_string)
                .map_err(ConfigError::CreateConfigFile)?;
        }

        let signing_key_path = self.as_ref().join(SIGNING_KEY);
        if !signing_key_path.exists() {
            // 기본 서명 키
            let signing_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
            fs::write(signing_key_path, signing_key).map_err(ConfigError::CreatePrivateKeyFile)?;
        }

        info!("Config directory at {:?}", self.as_ref());
        Ok(())
    }
    /// Initialize SKDE parameter file with default values if it doesn't exist.
    /// This is only used in authority nodes.
    pub fn init_skde_params_if_missing(&self) {
        let skde_path = self.as_ref().join("skde_params.json");

        // Skip if the file already exists
        if skde_path.exists() {
            return;
        }

        // Generate SKDE parameters
        let default_params = default_skde_params();

        // Serialize to JSON (POC: unwrap used)
        let serialized = serde_json::to_string_pretty(&default_params).unwrap();

        // Write to file (POC: unwrap used)
        fs::write(&skde_path, serialized).unwrap();

        info!("Default SKDE params written to {:?}", skde_path);
    }
}

#[derive(Debug, Parser, Serialize, Deserialize)]
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

    #[doc = "Set the solver rpc url"]
    #[clap(long = "solver-rpc-url")]
    pub solver_rpc_url: Option<String>,

    #[doc = "Set the leader cluster rpc url"]
    #[clap(long = "leader-cluster-rpc-url")]
    pub leader_cluster_rpc_url: Option<String>,

    #[doc = "Set the leader solver rpc url "]
    #[clap(long = "leader-slover-rpc-url")]
    pub leader_solver_rpc_url: Option<String>,

    #[doc = "Set the solver solver rpc url"]
    #[clap(long = "solver-slover-rpc-url")]
    pub solver_solver_rpc_url: Option<String>,

    #[doc = "Set the authority rpc url (used by leader node at startup)"]
    #[clap(long = "authority-rpc-url")]
    pub authority_rpc_url: Option<String>,

    #[doc = "Set the node role (leader, committee, solver, verifier)"]
    #[clap(long = "role")]
    pub role: Option<String>,

    #[doc = "Set the radius foundation address"]
    #[clap(long = "radius-foundation-address")]
    pub radius_foundation_address: Option<String>,

    #[doc = "Set the chain type (for verifying signature for foundation address)"]
    #[clap(long = "chain-type")]
    pub chain_type: Option<String>,

    #[doc = "Set session cycle"]
    #[clap(long = "session-cycle")]
    pub session_cycle: Option<u64>,
}

impl Default for ConfigOption {
    fn default() -> Self {
        Self {
            path: Some(ConfigPath::default().as_ref().into()),

            external_rpc_url: Some(DEFAULT_EXTERNAL_RPC_URL.into()),
            internal_rpc_url: Some(DEFAULT_INTERNAL_RPC_URL.into()),
            cluster_rpc_url: Some(DEFAULT_CLUSTER_RPC_URL.into()),
            solver_rpc_url: None,
            leader_cluster_rpc_url: None,
            leader_solver_rpc_url: None,
            solver_solver_rpc_url: None,
            authority_rpc_url: None,
            role: None,
            radius_foundation_address: Some(DEFAULT_RADIUS_FOUNDATION_ADDRESS.into()),
            chain_type: Some(DEFAULT_CHAIN_TYPE.into()),
            session_cycle: Some(DEFAULT_SESSION_CYCLE_MS),
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

        set_toml_comment(&mut toml_string, "Set solver rpc url");
        set_toml_name_value(&mut toml_string, "solver_rpc_url", &self.solver_rpc_url);

        set_toml_comment(&mut toml_string, "Set leader cluster rpc url");
        set_toml_name_value(
            &mut toml_string,
            "leader_cluster_rpc_url",
            &self.leader_cluster_rpc_url,
        );

        set_toml_comment(&mut toml_string, "Set leader solver rpc url");
        set_toml_name_value(
            &mut toml_string,
            "leader_solver_rpc_url",
            &self.leader_solver_rpc_url,
        );

        set_toml_comment(&mut toml_string, "Set solver solver rpc url");
        set_toml_name_value(
            &mut toml_string,
            "solver_solver_rpc_url",
            &self.solver_solver_rpc_url,
        );

        set_toml_comment(
            &mut toml_string,
            "Set authority rpc url (used by leader node at startup)",
        );
        set_toml_name_value(
            &mut toml_string,
            "authority_rpc_url",
            &self.authority_rpc_url,
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

        set_toml_comment(&mut toml_string, "Set session cycle");
        set_toml_name_value(&mut toml_string, "session_cycle", &self.session_cycle);

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

        if other.solver_rpc_url.is_some() {
            self.solver_rpc_url.clone_from(&other.solver_rpc_url);
        }

        if other.leader_cluster_rpc_url.is_some() {
            self.leader_cluster_rpc_url
                .clone_from(&other.leader_cluster_rpc_url);
        }

        if other.leader_solver_rpc_url.is_some() {
            self.leader_solver_rpc_url
                .clone_from(&other.leader_solver_rpc_url);
        }

        if other.solver_solver_rpc_url.is_some() {
            self.solver_solver_rpc_url
                .clone_from(&other.solver_solver_rpc_url);
        }

        if other.authority_rpc_url.is_some() {
            self.authority_rpc_url.clone_from(&other.authority_rpc_url);
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

        if other.session_cycle.is_some() {
            self.session_cycle.clone_from(&other.session_cycle);
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
