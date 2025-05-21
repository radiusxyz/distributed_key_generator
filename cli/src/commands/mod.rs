pub mod node;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Starts the node
    Node(Box<node::NodeCommand>),
    /// Create private key
    #[cfg(feature = "experimental")]
    CreatePrivateKey(Box<create_private_key::CreatePrivateKeyCommand>),
}
