pub mod key;
pub mod signature;
pub mod time;

use radius_sdk::signature::Address;

pub fn short_addr(address: &Address) -> String {
    let hex = address.as_hex_string();
    if hex.len() < 10 {
        hex
    } else {
        (hex[..6]).to_string()
    }
}