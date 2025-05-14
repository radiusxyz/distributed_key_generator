
mod key;
mod key_generator;
pub mod error;
pub mod state;
pub use key::*;
pub use key_generator::*;

mod primitives {
    pub use serde::{Deserialize, Serialize};
}

pub const CONFIG_FILE_NAME: &str = "Config.toml";
pub const SIGNING_KEY: &str = "signing_key";
