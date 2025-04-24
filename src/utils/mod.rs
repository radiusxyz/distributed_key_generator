use std::time::{SystemTime, UNIX_EPOCH};

use radius_sdk::signature::Address;

pub fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

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
