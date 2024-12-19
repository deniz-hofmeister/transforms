#[cfg(test)]
mod timestamp_tests {
    use crate::{errors::TimestampError, time::Timestamp};

    #[test]
    fn creation() {
        let _t = Timestamp { nanoseconds: 1 };
    }

    #[test]
    fn ordering() {
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
        assert_eq!(timestamp.as_seconds().unwrap(), 1.5);

        let timestamp = Timestamp { nanoseconds: 0 };
        assert_eq!(timestamp.as_seconds().unwrap(), 0.0);

        let timestamp = Timestamp {
            nanoseconds: 1_000_000_000,
        };
        assert_eq!(timestamp.as_seconds().unwrap(), 1.0);
    }

    #[test]
    fn as_seconds_accuracy_loss() {
        let timestamp = Timestamp {
            nanoseconds: u128::MAX - 1,
        };
        assert!(matches!(
            timestamp.as_seconds(),
            Err(TimestampError::AccuracyLoss)
        ));
    }
}
