use std::{fs, path::PathBuf};

use radius_sdk::signature::{ChainType, PrivateKeySigner, Signature};
use serde::{Deserialize, Serialize};
use skde::delay_encryption::{setup, SkdeParams};
use tracing::{info, warn};

use crate::ConfigPath;

// Constants for SKDE setup parameters
// TODO: Getting constants in a form of json file(?)
const DEFAULT_TIME_PARAM_T: u32 = 4;
const DEFAULT_GENERATOR: u32 = 4;
const DEFAULT_MAX_SEQUENCER_NUMBER: u32 = 2;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedSkdeParams {
    pub params: SkdeParams,
    pub signature: Signature,
}

// TODO: Add error handling
pub fn run_setup_skde_params(path: &ConfigPath) {
    let config_dir: PathBuf = path.as_ref().into();
    let skde_path = config_dir.join("skde_params.json");
    let signing_key_path = config_dir.join("signing_key");

    if skde_path.exists() {
        warn!("SKDE parameter file already exists: {:?}", skde_path);
        return;
    }

    let signing_key_hex = match fs::read_to_string(&signing_key_path) {
        Ok(key) => key.trim().to_string(),
        Err(e) => {
            warn!("Failed to read signing key from {:?}: {}", signing_key_path, e);
            return;
        }
    };

    let signer = match PrivateKeySigner::from_str(ChainType::Ethereum, &signing_key_hex) {
        Ok(signer) => signer,
        Err(e) => {
            warn!("Failed to create signer from signing key: {:?}", e);
            return;
        }
    };

    let params = setup(
        DEFAULT_TIME_PARAM_T,
        DEFAULT_GENERATOR.into(),
        DEFAULT_MAX_SEQUENCER_NUMBER.into(),
    );

    let signature = signer.sign_message(&params).unwrap();

    let signed_params = SignedSkdeParams { params, signature };

    let serialized = serde_json::to_string_pretty(&signed_params).unwrap();
    fs::write(&skde_path, serialized).unwrap();

    info!("Successfully generated SKDE params at {:?}", skde_path);
}
