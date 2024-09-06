use std::{fs, path::PathBuf};

use radius_sequencer_sdk::signature::ChainType;
use serde::{Deserialize, Serialize};

use super::{ConfigOption, ConfigPath, CONFIG_FILE_NAME, DATABASE_DIR_NAME, SIGNING_KEY};
use crate::{
    error::Error,
    types::{Address, SigningKey},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    path: PathBuf,

    external_rpc_url: String,
    internal_rpc_url: String,
    cluster_rpc_url: String,

    signing_key: SigningKey,

    radius_foundation_address: Address,
    chain_type: ChainType,
    // key_generate_delay_second: u64,
    // key_aggregate_delay_second: u64,
}

impl Config {
    pub fn load(config_option: &mut ConfigOption) -> Result<Self, Error> {
        let config_path = match config_option.path.as_mut() {
            Some(config_path) => config_path.clone(),
            None => {
                let config_path: PathBuf = ConfigPath::default().as_ref().into();
                config_option.path = Some(config_path.clone());
                config_path
            }
        };

        // Read config file
        let config_file_path = config_path.join(CONFIG_FILE_NAME);
        let config_string =
            fs::read_to_string(&config_file_path).map_err(|_| Error::LoadConfigOption)?;

        // Parse String to TOML String
        let config_file: ConfigOption =
            toml::from_str(&config_string).map_err(|_| Error::ParseTomlString)?;

        // Merge configs from CLI input
        let merged_config_option = config_file.merge(config_option);

        // Read signing key
        let signing_key_path = config_path.join(SIGNING_KEY);
        let signing_key = SigningKey::from(fs::read_to_string(signing_key_path).unwrap());

        Ok(Config {
            path: config_path,
            external_rpc_url: merged_config_option.external_rpc_url.unwrap(),
            internal_rpc_url: merged_config_option.internal_rpc_url.unwrap(),
            cluster_rpc_url: merged_config_option.cluster_rpc_url.unwrap(),
            signing_key,
            radius_foundation_address: merged_config_option
                .radius_foundation_address
                .unwrap()
                .into(),
            // TODO: stompesi
            // chain_type: merged_config_option.chain_type.unwrap().into(),
            chain_type: ChainType::Ethereum,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn database_path(&self) -> PathBuf {
        self.path.join(DATABASE_DIR_NAME)
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub fn radius_foundation_address(&self) -> &Address {
        &self.radius_foundation_address
    }

    pub fn chain_type(&self) -> &ChainType {
        &self.chain_type
    }

    pub fn address(&self) -> Address {
        self.signing_key().get_address()
    }

    pub fn external_rpc_url(&self) -> &String {
        &self.external_rpc_url
    }

    pub fn internal_rpc_url(&self) -> &String {
        &self.internal_rpc_url
    }

    // pub fn key_generate_delay_second(&self) -> &u64 {
    //     &self.key_generate_delay_second
    // }

    // pub fn key_aggregate_delay_second(&self) -> &u64 {
    //     &self.key_aggregate_delay_second
    // }

    pub fn cluster_rpc_url(&self) -> &String {
        &self.cluster_rpc_url
    }
}
