#[cfg(test)]
mod timestamp_tests {
    use crate::{errors::TimeError, time::Timestamp};
    use approx::assert_relative_eq;

    #[test]
    fn creation() {
        let _ = Timestamp::from_nanos(1);
    }

    #[test]
    fn ordering() {
        let t1 = Timestamp::from_nanos(1);
        let t2 = Timestamp::from_nanos(2);
        let t3 = Timestamp::from_nanos(2);

        assert!(t1 < t2);
        assert!(t2 > t1);
        assert_eq!(t2, t3);
        assert!(t2 >= t1);
        assert!(t1 <= t2);
    }

    #[test]
    fn as_seconds() {
        let timestamp = Timestamp::from_nanos(1_500_000_000);
        assert_relative_eq!(timestamp.as_seconds().unwrap(), 1.5);

        let timestamp = Timestamp::zero();
        assert_relative_eq!(timestamp.as_seconds().unwrap(), 0.0);

        let timestamp = Timestamp::from_nanos(1_000_000_000);
        assert_relative_eq!(timestamp.as_seconds().unwrap(), 1.0);
    }

    #[test]
    fn as_seconds_accuracy_loss() {
        let timestamp = Timestamp::from_nanos(u128::MAX - 1);
        assert!(matches!(
            timestamp.as_seconds(),
            Err(TimeError::AccuracyLoss)
        ));
    }

    #[test]
    #[cfg(feature = "std")]
    fn now_returns_a_dynamic_wall_clock_time() {
        use crate::time::TimePoint;

        let now = Timestamp::now();
        assert!(now.t > 0);
        assert!(!now.is_static());
    }

    #[test]
    fn as_seconds_accuracy_boundary_is_2_pow_53_nanos() {
        assert!(Timestamp::from_nanos(1 << 53).as_seconds().is_ok());
        assert!(Timestamp::from_nanos((1 << 53) + 1).as_seconds().is_err());

        // Best-effort conversions keep working beyond the boundary.
        let big = Timestamp::from_nanos((1 << 53) + 1);
        assert!(big.as_seconds_lossy().is_finite());
    }
}
