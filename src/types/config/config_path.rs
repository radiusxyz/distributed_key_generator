use std::{
    env, fs,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::{Deserialize, Serialize};
use crate::util::clear_dir;

use crate::types::{
    config::{config_option::ConfigOption, ConfigError},
    CONFIG_FILE_NAME, SIGNING_KEY,
};

#[derive(Debug, Deserialize, Parser, Serialize)]
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
        let path = PathBuf::from(env::var("HOME").unwrap())
            .join(super::DEFAULT_HOME_PATH)
            .to_str()
            .unwrap()
            .to_string();

        Self { path }
    }
}

impl ConfigPath {
    pub fn init(&self) -> Result<(), ConfigError> {
        // Remove the directory if it exists.
        if self.as_ref().exists() {
            clear_dir(self).map_err(ConfigError::RemoveConfigDirectory)?;
        }

        // Create the directory
        fs::create_dir_all(self).map_err(ConfigError::CreateConfigDirectory)?;

        // Create config file
        let config_file_path = self.as_ref().join(CONFIG_FILE_NAME);
        let config_toml_string = ConfigOption::default().get_toml_string();
        fs::write(config_file_path, config_toml_string).map_err(ConfigError::CreateConfigFile)?;

        // Generate a sign key.
        let signing_key_path = self.as_ref().join(SIGNING_KEY);
        // TODO: Generate a sign key.
        let signing_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        fs::write(signing_key_path, signing_key).map_err(ConfigError::CreatePrivateKeyFile)?;

        tracing::info!("Created a sign key {:?}", signing_key);
        tracing::info!("Created a new config directory at {:?}", self.as_ref());
        Ok(())
    }
}
