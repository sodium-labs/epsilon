use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub mod sql;
pub mod url;

pub fn get_timestamp() -> Duration {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch
}

/// Safe slice a string
pub fn safe_slice(s: &str, max_bytes: usize) -> &str {
    let mut end = s.len();

    for (idx, c) in s.char_indices() {
        if idx + c.len_utf8() > max_bytes {
            end = idx;
            break;
        }
    }

    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_slice() {
        assert_eq!(safe_slice("abc123", 6), "abc123");
        assert_eq!(safe_slice("", 6), "");
        assert_eq!(safe_slice("abc123", 5), "abc12");
        assert_eq!(safe_slice("abc123", 7), "abc123");
        assert_eq!(safe_slice("abc123def", 6), "abc123");
        assert_eq!(safe_slice("abc123def", 0), "");
        assert_eq!(safe_slice("", 0), "");
        assert_eq!(safe_slice("abc12é", 6), "abc12");
        assert_eq!(safe_slice("éééééé", 6), "ééé");
    }
}
