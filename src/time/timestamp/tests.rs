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
        let timestamp = Timestamp::from_nanos(u64::MAX - 1);
        assert!(matches!(
            timestamp.as_seconds(),
            Err(TimeError::AccuracyLoss)
        ));
    }

    #[test]
    #[cfg(feature = "std")]
    fn now_returns_a_positive_wall_clock_time() {
        let now = Timestamp::now();
        assert!(now.t > 0);
    }

    #[test]
    #[cfg(feature = "std")]
    fn try_now_returns_the_current_time_without_panicking() {
        let now = Timestamp::try_now().unwrap();
        assert!(now.t > 0);
    }

    #[test]
    fn checked_sub_below_zero_underflows() {
        use crate::time::TimePoint;
        use core::time::Duration;

        let t = Timestamp::from_nanos(1);
        assert!(matches!(
            t.checked_sub(Duration::from_nanos(2)),
            Err(TimeError::DurationUnderflow)
        ));
    }

    #[test]
    fn adding_beyond_the_representable_range_overflows() {
        use core::time::Duration;

        let t = Timestamp::from_nanos(u64::MAX);
        assert!(matches!(
            t + Duration::from_nanos(1),
            Err(TimeError::DurationOverflow)
        ));
    }

    #[test]
    fn a_duration_wider_than_the_timestamp_range_is_rejected_both_ways() {
        use core::time::Duration;

        // `Duration` counts seconds in a u64, so it can express spans no
        // `Timestamp` can hold. Both directions must report the range
        // failure instead of truncating the nanosecond count.
        let wide = Duration::from_secs(u64::MAX);
        let t = Timestamp::from_nanos(1_000_000_000);

        assert!(matches!(t + wide, Err(TimeError::DurationOverflow)));
        assert!(matches!(t - wide, Err(TimeError::DurationUnderflow)));
    }

    #[test]
    fn the_top_of_the_range_is_an_ordinary_timestamp() {
        // u64 nanoseconds span ~584 years; the last one is ordinary data,
        // not a sentinel.
        let top = Timestamp::from_nanos(u64::MAX);
        assert_eq!(top.as_nanos(), u64::MAX);
        assert!(top > Timestamp::zero());
    }

    #[test]
    fn subtraction_spans_the_whole_range() {
        use core::time::Duration;

        // The widest possible span is a valid `Duration` (~584 years), so
        // the subtraction has no overflow arm to take.
        let span = Timestamp::from_nanos(u64::MAX) - Timestamp::zero();
        assert_eq!(span.unwrap(), Duration::from_nanos(u64::MAX));

        let zero = Timestamp::from_nanos(7) - Timestamp::from_nanos(7);
        assert_eq!(zero.unwrap(), Duration::ZERO);

        let backwards = Timestamp::zero() - Timestamp::from_nanos(1);
        assert!(matches!(backwards, Err(TimeError::DurationUnderflow)));
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
