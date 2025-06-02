use clap::Args;
use std::path::PathBuf;
use skde::delay_encryption::setup;
use std::fs;
use std::time::Instant;

const DEFAULT_TIME_PARAM_T: u32 = 4;
const DEFAULT_GENERATOR: u32 = 4;
const DEFAULT_MAX_SEQUENCER_NUMBER: u32 = 2;

#[derive(Debug, Args)]
pub struct SkdeArgs {
    #[arg(long = "skde.path")]
    pub path: Option<PathBuf>,
    #[arg(long = "skde.generator", default_value_t = DEFAULT_GENERATOR)]
    pub generator: u32,
    #[arg(long = "skde.time", default_value_t = DEFAULT_TIME_PARAM_T)]
    pub time_param_t: u32,
    #[arg(long = "skde.max-sequencer", default_value_t = DEFAULT_MAX_SEQUENCER_NUMBER)]
    pub max_sequencer_number: u32,
}

pub fn run_skde_inner(cli: SkdeArgs) {
    println!("Setting up skde params...");
    let start = Instant::now();
    let params = setup(cli.time_param_t, cli.generator.into(), cli.max_sequencer_number.into());
    let path = cli.path.unwrap_or(PathBuf::from(format!("./tmp/authority/trusted_setup/trusted_setup.json")));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let serialized = serde_json::to_string_pretty(&params).unwrap();
    fs::write(path, serialized).unwrap();
    let duration = start.elapsed();
    println!("Skde params setup in {:?}", duration);
}