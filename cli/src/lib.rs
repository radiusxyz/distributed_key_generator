
use std::path::PathBuf;

use clap::{Parser, Args};

mod config;
pub use config::*;

mod command;
pub use command::run;

mod commands;
use commands::*;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[doc = "Path to configuration file (TOML format)"]
    #[arg(long = "config", global = true)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    pub fn init() -> Self {
        Cli::parse()
    }
}