use radius_sdk::signature::Address;

use crate::{Config, SessionId};

// A extension method to convert an address to a short format
pub trait AddressExt {
    fn to_short(&self) -> String;
}

impl AddressExt for Address {
    fn to_short(&self) -> String {
        let hex = self.as_hex_string();
        if hex.len() < 10 {
            hex
        } else {
            format!("{}", &hex[..6])
        }
    }
}

pub fn log_prefix_role_and_address(config: &Config) -> String {
    format!("[{}][{}]", config.role(), config.address().to_short(),)
}

pub fn log_prefix_with_session_id(config: &Config, session_id: &SessionId) -> String {
    format!(
        "[{}][{}][session:{}]",
        config.role(),
        config.address().to_short(),
        session_id.as_u64()
    )
}
