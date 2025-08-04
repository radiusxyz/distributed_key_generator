use clap::{Parser, Subcommand};

#[cfg(feature = "skde")]
mod skde;
#[cfg(feature = "skde")]
pub use skde::*;

#[derive(Debug, Parser)]
pub struct TrustedSetupCommand {
    #[command(subcommand)]
    pub method: Method,
}


#[derive(Debug, Subcommand)]
pub enum Method {
    #[cfg(feature = "skde")]
    Skde(SkdeArgs),
}