use std::{fs, path::PathBuf};

use skde::delay_encryption::setup;

use crate::ConfigPath;

// Constants for SKDE setup parameters
// TODO: Getting constants in a form of json file(?)
const DEFAULT_TIME_PARAM_T: u32 = 4;
const DEFAULT_GENERATOR: u32 = 4;
const DEFAULT_MAX_SEQUENCER_NUMBER: u32 = 2;

// TODO: Add error handling
pub fn run_setup_skde_params(path: &ConfigPath) {
    let config_dir: PathBuf = path.as_ref().into();
    let skde_path = config_dir.join("skde_params.json");

    if skde_path.exists() {
        tracing::warn!("SKDE parameter file already exists: {:?}", skde_path);
        return;
    }

    let t = DEFAULT_TIME_PARAM_T;
    let g = DEFAULT_GENERATOR.into();
    let max = DEFAULT_MAX_SEQUENCER_NUMBER.into();

    let params = setup(t, g, max);
    // TODO: Add sign
    let serialized = serde_json::to_string_pretty(&params).unwrap();
    fs::write(&skde_path, serialized).unwrap();

    tracing::info!("Successfully generated SKDE params at {:?}", skde_path);
}
