use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    pub nanoseconds: u128,
}

impl Timestamp {
    pub fn now() -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        Timestamp {
            nanoseconds: now.as_nanos(),
        }
    }
}

#[test]
fn test_timestamp() {
    let t = Timestamp::now();
    assert!(t.nanoseconds > 0);
}
