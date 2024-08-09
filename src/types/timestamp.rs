use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn timestamp_creation() {
        let _t = Timestamp { nanoseconds: 1 };
    }
    #[test]
    fn timestamp_now() {
        let t = Timestamp::now();
        assert!(t.nanoseconds > 0);
    }

    #[test]
    fn timestamp_ordering() {
        let t1 = Timestamp { nanoseconds: 1 };
        let t2 = Timestamp { nanoseconds: 2 };
        let t3 = Timestamp { nanoseconds: 2 };

        assert!(t1 < t2);
        assert!(t2 > t1);
        assert!(t2 == t3);
        assert!(t2 >= t1);
        assert!(t1 <= t2);
    }
}
