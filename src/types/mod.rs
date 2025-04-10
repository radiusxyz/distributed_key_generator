pub mod config;
mod key;
mod key_generator;
mod session;

pub use config::*;
pub use key::*;
pub use key_generator::*;
pub use session::*;

pub(crate) mod prelude {
    pub use radius_sdk::kvstore::KvStoreError;
    pub use serde::{Deserialize, Serialize};
}

pub const CONFIG_FILE_NAME: &str = "Config.toml";
pub const SIGNING_KEY: &str = "signing_key";

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Hash)]
pub struct SessionId(u64);

impl SessionId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for SessionId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
