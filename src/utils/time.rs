use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};

pub fn get_current_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub fn to_iso8601(timestamp_ms: u128) -> String {
    let seconds = (timestamp_ms / 1000) as i64;
    let millis = (timestamp_ms % 1000) as u32;

    let datetime = match DateTime::<Utc>::from_timestamp(seconds, millis * 1_000_000) {
        Some(dt) => dt,
        None => return format!("Invalid timestamp: {}", timestamp_ms),
    };

    datetime.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_iso8601() {
        let timestamp = 1672531200123u128;
        let formatted = to_iso8601(timestamp);
        assert_eq!(formatted, "2023-01-01T00:00:00.123Z");
    }
}
