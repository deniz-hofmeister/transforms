use std::ops::Sub;
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

    pub fn as_seconds(&self) -> f64 {
        self.nanoseconds as f64 / 1_000_000_000.0
    }
}

impl Sub<u128> for Timestamp {
    type Output = Self;

    fn sub(
        self,
        other: u128,
    ) -> Self::Output {
        Timestamp {
            nanoseconds: self.nanoseconds.saturating_sub(other),
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

    #[test]
    fn as_seconds() {
        let timestamp = Timestamp {
            nanoseconds: 1_500_000_000,
        };
        assert_eq!(timestamp.as_seconds(), 1.5);

        let timestamp = Timestamp { nanoseconds: 0 };
        assert_eq!(timestamp.as_seconds(), 0.0);

        let timestamp = Timestamp {
            nanoseconds: 1_000_000_000,
        };
        assert_eq!(timestamp.as_seconds(), 1.0);
    }
}
