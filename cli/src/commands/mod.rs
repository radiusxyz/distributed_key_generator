pub mod node;
pub mod trusted_setup;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Starts the node
    Node(Box<node::NodeCommand>),
    /// Set trusted setup
    TrustedSetup(Box<trusted_setup::TrustedSetupCommand>),
}
