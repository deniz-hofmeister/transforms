#[cfg(test)]
mod timestamp_tests {
    use crate::{errors::TimestampError, time::Timestamp};

    #[test]
    fn creation() {
        let _t = Timestamp { time: 1 };
    }

    #[test]
    fn ordering() {
        let t1 = Timestamp { time: 1 };
        let t2 = Timestamp { time: 2 };
        let t3 = Timestamp { time: 2 };

        assert!(t1 < t2);
        assert!(t2 > t1);
        assert!(t2 == t3);
        assert!(t2 >= t1);
        assert!(t1 <= t2);
    }

    #[test]
    fn as_seconds() {
        let timestamp = Timestamp {
            time: 1_500_000_000,
        };
        assert_eq!(timestamp.as_seconds().unwrap(), 1.5);

        let timestamp = Timestamp { time: 0 };
        assert_eq!(timestamp.as_seconds().unwrap(), 0.0);

        let timestamp = Timestamp {
            time: 1_000_000_000,
        };
        assert_eq!(timestamp.as_seconds().unwrap(), 1.0);
    }

    #[test]
    fn as_seconds_accuracy_loss() {
        let timestamp = Timestamp {
            time: u128::MAX - 1,
        };
        assert!(matches!(
            timestamp.as_seconds(),
            Err(TimestampError::AccuracyLoss)
        ));
    }
}
