use clap::{Parser, Subcommand};

mod skde;
pub use skde::*;

#[derive(Debug, Parser)]
pub struct TrustedSetupCommand {
    #[command(subcommand)]
    pub method: Method,
}


#[derive(Debug, Subcommand)]
pub enum Method {
    Skde(SkdeArgs),
}