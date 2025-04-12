use std::time::{SystemTime, UNIX_EPOCH};

/// Get the current timestamp in bigint format
pub fn get_sql_timestamp() -> i64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_millis().try_into().unwrap()
}
