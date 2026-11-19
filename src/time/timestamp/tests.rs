//! Unit tests.

use super::{Timestamp, nanos_to_f64};
use alloc::format;
use core::time::Duration;
use std::collections::HashSet;

/// `i64::MAX` nanoseconds as the unsigned count a `Duration` is built from.
const I64_MAX_NANOS: u64 = i64::MAX.unsigned_abs();

fn ts(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

#[test]
fn from_nanos_and_as_nanos_round_trip_the_whole_range() {
    for nanos in [i64::MIN, -1_000_000_000, -1, 0, 1, 1_000_000_000, i64::MAX] {
        assert_eq!(ts(nanos).as_nanos(), nanos);
    }
}

#[test]
fn from_nanos_and_as_nanos_are_usable_in_a_const_context() {
    const T: Timestamp = Timestamp::from_nanos(-5);
    const NANOS: i64 = T.as_nanos();
    const TOP: i64 = Timestamp::MAX.as_nanos();

    assert_eq!(NANOS, -5);
    assert_eq!(TOP, i64::MAX);
}

#[test]
fn max_is_the_latest_instant_and_an_ordinary_one() {
    assert_eq!(Timestamp::MAX, ts(i64::MAX));
    assert_eq!(Timestamp::MAX.as_nanos(), i64::MAX);
    assert!(Timestamp::MAX > ts(0));
    assert!(Timestamp::MAX > ts(i64::MAX - 1));
    assert_eq!(Timestamp::MAX.checked_add(Duration::from_nanos(1)), None);
}

#[test]
fn ordering_follows_the_signed_nanosecond_count() {
    let t1 = ts(1);
    let t2 = ts(2);
    let t3 = ts(2);

    assert!(t1 < t2);
    assert!(t2 > t1);
    assert_eq!(t2, t3);
    assert!(t2 >= t1);
    assert!(t1 <= t2);

    // Instants before the epoch sort before it: the count is signed.
    assert!(ts(-1) < ts(0));
    assert!(ts(i64::MIN) < ts(-1));
    assert!(ts(i64::MIN) < ts(i64::MAX));
    assert_eq!(ts(-3).max(ts(2)), ts(2));
    assert_eq!(ts(-3).min(ts(2)), ts(-3));

    let mut instants = [ts(5), ts(-7), ts(i64::MAX), ts(0), ts(i64::MIN)];
    instants.sort();
    assert_eq!(instants, [ts(i64::MIN), ts(-7), ts(0), ts(5), ts(i64::MAX)]);
}

#[test]
fn equal_instants_hash_alike() {
    let mut seen = HashSet::new();

    assert!(seen.insert(ts(1)));
    assert!(seen.insert(ts(-1)));
    assert!(seen.insert(ts(0)));
    assert!(!seen.insert(ts(1)));
    assert!(!seen.insert(Timestamp::from_nanos(-1)));
    assert_eq!(seen.len(), 3);
    assert!(seen.contains(&ts(0)));
    assert!(!seen.contains(&ts(2)));
}

#[test]
fn checked_add_moves_forward_exactly() {
    assert_eq!(ts(1).checked_add(Duration::from_nanos(2)), Some(ts(3)));
    assert_eq!(ts(-5).checked_add(Duration::from_nanos(2)), Some(ts(-3)));
    assert_eq!(ts(-5).checked_add(Duration::from_nanos(7)), Some(ts(2)));
    assert_eq!(
        ts(1_700_000_000_123_456_789).checked_add(Duration::from_millis(5)),
        Some(ts(1_700_000_000_128_456_789))
    );
}

#[test]
fn checked_sub_moves_backward_exactly_and_crosses_the_epoch() {
    assert_eq!(ts(3).checked_sub(Duration::from_nanos(2)), Some(ts(1)));
    // 2.4's unsigned timestamp underflowed here; a signed one does not.
    assert_eq!(ts(1).checked_sub(Duration::from_nanos(2)), Some(ts(-1)));
    assert_eq!(
        ts(1_000).checked_sub(Duration::from_nanos(1_500)),
        Some(ts(-500))
    );
}

#[test]
fn a_zero_duration_leaves_every_instant_unchanged() {
    for nanos in [i64::MIN, -1, 0, 1, i64::MAX] {
        assert_eq!(ts(nanos).checked_add(Duration::ZERO), Some(ts(nanos)));
        assert_eq!(ts(nanos).checked_sub(Duration::ZERO), Some(ts(nanos)));
    }
}

#[test]
fn checked_arithmetic_crosses_the_whole_range_in_one_step() {
    // i64::MIN + (2^64 - 1) = i64::MAX: the widest step there is.
    let whole_range = Duration::from_nanos(u64::MAX);

    assert_eq!(ts(i64::MIN).checked_add(whole_range), Some(Timestamp::MAX));
    assert_eq!(Timestamp::MAX.checked_sub(whole_range), Some(ts(i64::MIN)));

    // One nanosecond more does not fit, from either end.
    let too_wide = whole_range + Duration::from_nanos(1);
    assert_eq!(ts(i64::MIN).checked_add(too_wide), None);
    assert_eq!(Timestamp::MAX.checked_sub(too_wide), None);

    // Starting one nanosecond inside the range leaves no room either.
    assert_eq!(ts(i64::MIN + 1).checked_add(whole_range), None);
    assert_eq!(ts(i64::MAX - 1).checked_sub(whole_range), None);
}

#[test]
fn checked_arithmetic_takes_durations_above_i64_max_nanoseconds() {
    // A duration of i64::MAX + 1 nanoseconds does not fit an i64. It is
    // still an exact step when the start leaves room for it.
    let above = Duration::from_nanos(I64_MAX_NANOS + 1);

    assert_eq!(ts(-1).checked_add(above), Some(Timestamp::MAX));
    assert_eq!(ts(0).checked_add(above), None);
    assert_eq!(ts(-2).checked_add(above), Some(ts(i64::MAX - 1)));
    assert_eq!(ts(i64::MIN).checked_add(above), Some(ts(0)));

    assert_eq!(ts(0).checked_sub(above), Some(ts(i64::MIN)));
    assert_eq!(ts(-1).checked_sub(above), None);
    assert_eq!(ts(1).checked_sub(above), Some(ts(i64::MIN + 1)));
    assert_eq!(Timestamp::MAX.checked_sub(above), Some(ts(-1)));

    // Exactly i64::MAX nanoseconds, the last duration an i64 holds.
    let at = Duration::from_nanos(I64_MAX_NANOS);
    assert_eq!(ts(0).checked_add(at), Some(Timestamp::MAX));
    assert_eq!(ts(1).checked_add(at), None);
    assert_eq!(ts(0).checked_sub(at), Some(ts(i64::MIN + 1)));
    assert_eq!(ts(-1).checked_sub(at), Some(ts(i64::MIN)));
    assert_eq!(ts(-2).checked_sub(at), None);
}

#[test]
fn checked_arithmetic_stops_at_the_ends_of_the_range() {
    assert_eq!(Timestamp::MAX.checked_add(Duration::from_nanos(1)), None);
    assert_eq!(ts(i64::MIN).checked_sub(Duration::from_nanos(1)), None);
    assert_eq!(
        ts(i64::MAX - 1).checked_add(Duration::from_nanos(1)),
        Some(Timestamp::MAX)
    );
    assert_eq!(
        ts(i64::MIN + 1).checked_sub(Duration::from_nanos(1)),
        Some(ts(i64::MIN))
    );
}

#[test]
fn a_duration_wider_than_the_timestamp_range_is_rejected_both_ways() {
    // `Duration` counts seconds in a u64, so it expresses spans no
    // `Timestamp` pair is apart. Neither direction truncates the count.
    for wide in [Duration::from_secs(u64::MAX), Duration::MAX] {
        for nanos in [i64::MIN, -1, 0, 1_000_000_000, i64::MAX] {
            assert_eq!(ts(nanos).checked_add(wide), None);
            assert_eq!(ts(nanos).checked_sub(wide), None);
        }
    }
}

#[test]
fn checked_duration_since_is_none_exactly_when_earlier_is_later() {
    let instants = [i64::MIN, i64::MIN + 1, -250, -1, 0, 1, 750, i64::MAX];

    for &later in &instants {
        for &earlier in &instants {
            let elapsed = ts(later).checked_duration_since(ts(earlier));
            assert_eq!(
                elapsed.is_none(),
                earlier > later,
                "{later} since {earlier}"
            );
        }
    }
}

#[test]
fn checked_duration_since_is_zero_for_equal_instants() {
    for nanos in [i64::MIN, -7, 0, 7, i64::MAX] {
        assert_eq!(
            ts(nanos).checked_duration_since(ts(nanos)),
            Some(Duration::ZERO)
        );
    }
}

#[test]
fn checked_duration_since_spans_the_whole_range_exactly() {
    // The widest span, 2^64 - 1 ns, is a valid `Duration`.
    assert_eq!(
        Timestamp::MAX.checked_duration_since(ts(i64::MIN)),
        Some(Duration::from_nanos(u64::MAX))
    );
    assert_eq!(ts(i64::MIN).checked_duration_since(Timestamp::MAX), None);

    assert_eq!(
        ts(750).checked_duration_since(ts(-250)),
        Some(Duration::from_nanos(1_000))
    );
    assert_eq!(ts(-250).checked_duration_since(ts(750)), None);
    assert_eq!(
        ts(0).checked_duration_since(ts(i64::MIN)),
        Some(Duration::from_nanos(I64_MAX_NANOS + 1))
    );
    assert_eq!(
        Timestamp::MAX.checked_duration_since(ts(0)),
        Some(Duration::from_nanos(I64_MAX_NANOS))
    );
    assert_eq!(ts(0).checked_duration_since(ts(1)), None);
}

#[test]
fn checked_duration_since_inverts_checked_add() {
    for (start, step) in [
        (i64::MIN, u64::MAX),
        (-250, 1_000),
        (0, I64_MAX_NANOS),
        (1_700_000_000_123_456_789, 5_000_000),
    ] {
        let step = Duration::from_nanos(step);
        let end = ts(start).checked_add(step).unwrap();

        assert_eq!(end.checked_duration_since(ts(start)), Some(step));
        assert_eq!(end.checked_sub(step), Some(ts(start)));
    }
}

#[test]
fn display_prints_sign_magnitude_seconds_with_nine_fractional_digits() {
    assert_eq!(format!("{}", ts(0)), "0.000000000");
    assert_eq!(format!("{}", ts(1)), "0.000000001");
    assert_eq!(format!("{}", ts(-1)), "-0.000000001");
    assert_eq!(format!("{}", ts(-500_000_000)), "-0.500000000");
    assert_eq!(format!("{}", ts(1_500_000_000)), "1.500000000");
    assert_eq!(format!("{}", ts(-1_500_000_000)), "-1.500000000");
    assert_eq!(
        format!("{}", ts(1_783_400_000_123_456_789)),
        "1783400000.123456789"
    );
}

#[test]
fn display_prints_both_ends_of_the_range() {
    assert_eq!(format!("{}", ts(i64::MIN)), "-9223372036.854775808");
    assert_eq!(format!("{}", ts(i64::MAX)), "9223372036.854775807");
}

#[test]
fn display_ignores_width_fill_and_precision_flags() {
    let t = ts(-500_000_000);

    assert_eq!(format!("{t:30}"), "-0.500000000");
    assert_eq!(format!("{t:>30}"), "-0.500000000");
    assert_eq!(format!("{t:*<30}"), "-0.500000000");
    assert_eq!(format!("{t:.2}"), "-0.500000000");
    assert_eq!(format!("{t:030.3}"), "-0.500000000");
    assert_eq!(format!("{:+}", ts(1)), "0.000000001");
}

#[test]
fn debug_shows_the_nanosecond_count() {
    assert_eq!(format!("{:?}", ts(-5)), "Timestamp(-5)");
}

#[test]
fn ratio_between_is_the_exact_fraction_of_the_span() {
    let bits = |at: i64, from: i64, to: i64| ts(at).ratio_between(ts(from), ts(to)).to_bits();

    // The exact midpoint, at ordinary, negative and wall-clock instants.
    assert_eq!(bits(5, 0, 10), 0.5_f64.to_bits());
    assert_eq!(bits(0, -10, 10), 0.5_f64.to_bits());
    assert_eq!(bits(-15, -20, -10), 0.5_f64.to_bits());
    assert_eq!(
        bits(
            1_783_400_000_623_456_789,
            1_783_400_000_123_456_789,
            1_783_400_001_123_456_789
        ),
        0.5_f64.to_bits()
    );

    // Other fractions: each is the correctly rounded quotient of two
    // exactly converted spans.
    assert_eq!(bits(1, 0, 4), 0.25_f64.to_bits());
    assert_eq!(bits(-7, -10, -6), 0.75_f64.to_bits());
    assert_eq!(bits(1, 0, 3), (1.0_f64 / 3.0).to_bits());
    assert_eq!(bits(1, 0, 10), 0.1_f64.to_bits());
    assert_eq!(
        bits(1_000_000_001, 1_000_000_000, 4_000_000_000),
        (1.0_f64 / 3_000_000_000.0).to_bits()
    );
}

#[test]
fn ratio_between_spans_the_whole_range_without_overflow() {
    // The spans are unsigned differences: 2^63 over 2^64 - 1, which
    // converts to 2^64 and divides to exactly one half.
    assert_eq!(
        ts(0).ratio_between(ts(i64::MIN), Timestamp::MAX).to_bits(),
        0.5_f64.to_bits()
    );
    // One nanosecond into the widest span: 1 / 2^64.
    assert_eq!(
        ts(i64::MIN + 1)
            .ratio_between(ts(i64::MIN), Timestamp::MAX)
            .to_bits(),
        (1.0_f64 / 18_446_744_073_709_551_616.0).to_bits()
    );
}

#[test]
fn nanos_to_f64_is_the_correctly_rounded_conversion() {
    for (nanos, expected) in [
        (0_u64, 0.0_f64),
        (1, 1.0),
        // 2^32 - 1 and 2^32: either side of the split between the halves.
        (4_294_967_295, 4_294_967_295.0),
        (4_294_967_296, 4_294_967_296.0),
        // 2^53 is the last count with an exact neighbour; 2^53 + 1 is a
        // tie between 2^53 and 2^53 + 2 and rounds to the even one.
        (9_007_199_254_740_992, 9_007_199_254_740_992.0),
        (9_007_199_254_740_993, 9_007_199_254_740_992.0),
        (9_007_199_254_740_994, 9_007_199_254_740_994.0),
        (9_007_199_254_740_995, 9_007_199_254_740_996.0),
        // 2^64 - 1 rounds up to 2^64.
        (u64::MAX, 18_446_744_073_709_551_616.0),
    ] {
        assert_eq!(
            nanos_to_f64(nanos).to_bits(),
            expected.to_bits(),
            "{nanos} ns"
        );
    }
}
