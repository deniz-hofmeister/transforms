//! `Timestamp` through the public API: ordering, checked arithmetic,
//! formatting, and its use as the registry's time.
//!
//! The second half contrasts the checked functions with the hand-written
//! forms they replace. Each of those forms returns a wrong answer at an
//! ordinary timestamp, without an overflow and without a panic.

use core::time::Duration;
use transforms::{
    InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3,
};

const SECOND: i64 = 1_000_000_000;

fn ts(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

/// An edge with no rotation whose child sits at `(x, 0, 0)` in its parent.
fn edge(
    target: &str,
    source: &str,
    x: f64,
    stamp: Stamp,
) -> Transform {
    Transform::new(
        target,
        source,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        stamp,
    )
    .unwrap()
}

// ---- construction and ordering ----

#[test]
fn nanoseconds_round_trip_over_the_whole_signed_range() {
    for nanos in [i64::MIN, -SECOND, -1, 0, 1, SECOND, i64::MAX] {
        assert_eq!(ts(nanos).as_nanos(), nanos);
    }
    assert_eq!(Timestamp::MAX.as_nanos(), i64::MAX);
}

#[test]
fn instants_order_by_their_signed_nanosecond_count() {
    let mut instants = [ts(5), Timestamp::MAX, ts(-5), ts(0), ts(i64::MIN), ts(-6)];
    instants.sort();
    assert_eq!(
        instants,
        [ts(i64::MIN), ts(-6), ts(-5), ts(0), ts(5), Timestamp::MAX]
    );

    assert!(ts(-1) < ts(0));
    assert!(ts(0) < ts(1));
    assert_eq!(ts(7), ts(7));
    assert_ne!(ts(7), ts(-7));
    assert_eq!(ts(3).max(ts(-4)), ts(3));
    assert_eq!([ts(2), ts(9), ts(-3)].into_iter().max(), Some(ts(9)));
    assert_eq!([ts(2), ts(9), ts(-3)].into_iter().min(), Some(ts(-3)));
}

// ---- checked arithmetic ----

#[test]
fn checked_add_and_sub_shift_by_a_duration() {
    let t = ts(1_700_000_000_123_456_789);
    assert_eq!(
        t.checked_add(Duration::from_millis(5)),
        Some(ts(1_700_000_000_128_456_789))
    );
    assert_eq!(
        t.checked_sub(Duration::from_secs(1)),
        Some(ts(1_699_999_999_123_456_789))
    );
    assert_eq!(t.checked_add(Duration::ZERO), Some(t));
    assert_eq!(t.checked_sub(Duration::ZERO), Some(t));

    // Across the epoch, in both directions.
    assert_eq!(
        ts(1_000).checked_sub(Duration::from_nanos(1_500)),
        Some(ts(-500))
    );
    assert_eq!(
        ts(-500).checked_add(Duration::from_nanos(1_500)),
        Some(ts(1_000))
    );
}

#[test]
fn checked_add_and_sub_return_none_outside_the_range() {
    let one = Duration::from_nanos(1);
    assert_eq!(Timestamp::MAX.checked_add(one), None);
    assert_eq!(ts(i64::MIN).checked_sub(one), None);
    assert_eq!(ts(i64::MAX - 1).checked_add(one), Some(Timestamp::MAX));
    assert_eq!(ts(i64::MIN + 1).checked_sub(one), Some(ts(i64::MIN)));

    for t in [ts(i64::MIN), ts(-1), ts(0), ts(1), Timestamp::MAX] {
        assert_eq!(t.checked_add(Duration::MAX), None);
        assert_eq!(t.checked_sub(Duration::MAX), None);
    }
}

#[test]
fn checked_add_and_sub_span_the_whole_range_in_one_step() {
    // The distance between the two ends does not fit an `i64`. The
    // duration is not narrowed before it is applied.
    let whole_range = Duration::from_nanos(u64::MAX);
    assert_eq!(ts(i64::MIN).checked_add(whole_range), Some(Timestamp::MAX));
    assert_eq!(Timestamp::MAX.checked_sub(whole_range), Some(ts(i64::MIN)));
    assert_eq!(ts(i64::MIN + 1).checked_add(whole_range), None);
    assert_eq!(ts(i64::MAX - 1).checked_sub(whole_range), None);

    // One nanosecond more than `i64::MAX` nanoseconds.
    let just_over = Duration::from_nanos(1 << 63);
    assert_eq!(ts(-1).checked_add(just_over), Some(Timestamp::MAX));
    assert_eq!(ts(0).checked_add(just_over), None);
    assert_eq!(ts(0).checked_sub(just_over), Some(ts(i64::MIN)));
    assert_eq!(ts(-1).checked_sub(just_over), None);
}

#[test]
fn checked_duration_since_is_none_exactly_when_the_other_instant_is_later() {
    let cases = [
        (ts(750), ts(-250), Some(Duration::from_nanos(1_000))),
        (ts(-250), ts(750), None),
        (ts(5), ts(5), Some(Duration::ZERO)),
        (ts(-5), ts(-6), Some(Duration::from_nanos(1))),
        (ts(-6), ts(-5), None),
        (
            Timestamp::MAX,
            ts(i64::MIN),
            Some(Duration::from_nanos(u64::MAX)),
        ),
        (ts(i64::MIN), Timestamp::MAX, None),
        (ts(i64::MIN), ts(i64::MIN), Some(Duration::ZERO)),
    ];
    for (later, earlier, expected) in cases {
        assert_eq!(
            later.checked_duration_since(earlier),
            expected,
            "{later} since {earlier}"
        );
        assert_eq!(expected.is_none(), earlier > later);
    }
}

#[test]
fn checked_operations_are_inverse_to_each_other() {
    let t = ts(-123_456_789);
    let d = Duration::new(3, 500);
    let later = t.checked_add(d).unwrap();
    assert_eq!(later.checked_sub(d), Some(t));
    assert_eq!(later.checked_duration_since(t), Some(d));
}

// ---- Display ----

#[test]
fn display_prints_sign_magnitude_seconds_with_nine_fractional_digits() {
    for (nanos, expected) in [
        (0, "0.000000000"),
        (1, "0.000000001"),
        (-1, "-0.000000001"),
        (-500_000_000, "-0.500000000"),
        (999_999_999, "0.999999999"),
        (SECOND, "1.000000000"),
        (-SECOND, "-1.000000000"),
        (1_500_000_000, "1.500000000"),
        (1_700_000_000_123_456_789, "1700000000.123456789"),
        (i64::MAX, "9223372036.854775807"),
        (i64::MIN, "-9223372036.854775808"),
    ] {
        assert_eq!(ts(nanos).to_string(), expected);
    }
}

#[test]
fn display_keeps_adjacent_nanoseconds_apart() {
    // In seconds as `f64` these two print identically.
    let a = ts(1_700_000_000_000_000_001);
    let b = ts(1_700_000_000_000_000_002);
    assert_ne!(a.to_string(), b.to_string());
}

#[test]
fn display_ignores_width_fill_and_precision() {
    let t = ts(-1_500_000_000);
    assert_eq!(format!("{t:>20}"), "-1.500000000");
    assert_eq!(format!("{t:*<20}"), "-1.500000000");
    assert_eq!(format!("{t:.2}"), "-1.500000000");
    assert_eq!(format!("{t:020}"), "-1.500000000");
}

// ---- the hand-written forms the checked functions replace ----

#[test]
fn a_cast_difference_reads_a_stamp_ahead_of_the_clock_as_centuries_old() {
    let now = ts(1_700_000_000 * SECOND);
    let stamp = now.checked_add(Duration::from_millis(2)).unwrap();

    // The hand-written age. No overflow, no panic.
    let age = (now.as_nanos() - stamp.as_nanos()) as u64;
    let years_584 = 584 * 365 * 24 * 3600 * 1_000_000_000_u64;
    assert!(age > years_584, "{age}");

    // The checked form says the stamp has no age.
    assert_eq!(now.checked_duration_since(stamp), None);
    assert_eq!(
        stamp.checked_duration_since(now),
        Some(Duration::from_millis(2))
    );
}

#[test]
fn abs_diff_loses_which_instant_is_later() {
    let now = ts(1_700_000_000 * SECOND);
    let future = now.checked_add(Duration::from_secs(3)).unwrap();
    let past = now.checked_sub(Duration::from_secs(3)).unwrap();

    // A stamp 3 s in the future and one 3 s in the past have the same
    // hand-written age.
    assert_eq!(now.as_nanos().abs_diff(future.as_nanos()), 3_000_000_000);
    assert_eq!(now.as_nanos().abs_diff(past.as_nanos()), 3_000_000_000);

    // The checked form keeps the direction.
    assert_eq!(now.checked_duration_since(future), None);
    assert_eq!(
        now.checked_duration_since(past),
        Some(Duration::from_secs(3))
    );
}

#[test]
fn a_cast_timeout_turns_no_timeout_into_a_deadline_in_the_past() {
    let now = ts(1_700_000_000 * SECOND);
    let timeout = Duration::MAX;

    // The hand-written deadline. The cast keeps the low 64 bits.
    let deadline = now.as_nanos() + timeout.as_nanos() as i64;
    assert_eq!(deadline, now.as_nanos() - 1);
    assert!(ts(deadline) < now);

    // The checked form says the deadline is not representable.
    assert_eq!(now.checked_add(timeout), None);
}

#[test]
fn a_truncating_split_of_a_negative_instant_is_not_a_valid_stamp() {
    let half_second_before_epoch = ts(-500_000_000);
    let nanos = half_second_before_epoch.as_nanos();

    // `/` and `%` truncate toward zero, and the cast wraps the negative
    // remainder.
    let naive = ((nanos / SECOND) as i32, (nanos % SECOND) as u32);
    assert_eq!(naive, (0, 3_794_967_296));

    // The flooring split of the `Timestamp` documentation.
    let floored = (
        i32::try_from(nanos.div_euclid(SECOND)).unwrap(),
        u32::try_from(nanos.rem_euclid(SECOND)).unwrap(),
    );
    assert_eq!(floored, (-1, 500_000_000));
    // `Display` is neither: it prints sign and magnitude.
    assert_eq!(half_second_before_epoch.to_string(), "-0.500000000");
}

// ---- as the registry's time ----

#[test]
fn a_stored_sample_is_returned_at_its_own_instant() {
    let t = ts(SECOND);
    let transform = Transform::new(
        "map",
        "base",
        Vector3::new(1.0, 2.0, 3.0),
        Rotation::IDENTITY,
        Stamp::At(t),
    )
    .unwrap();
    let mut registry = Registry::unbounded();
    registry.insert(&transform).unwrap();

    assert_eq!(registry.lookup("map", "base", t), Ok(transform));
}

#[test]
fn a_static_transform_serves_every_instant_including_the_extremes() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("a", "b", 1.0, Stamp::Static))
        .unwrap();

    for probe in [
        i64::MIN,
        i64::MIN + 1,
        -1,
        0,
        12_345,
        i64::MAX - 1,
        i64::MAX,
    ] {
        // The result carries the requested instant.
        assert_eq!(
            registry.lookup("a", "b", ts(probe)),
            Ok(edge("a", "b", 1.0, Stamp::At(ts(probe))))
        );
    }
}

#[test]
fn static_and_dynamic_do_not_mix_in_either_order() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("a", "b", 1.0, Stamp::Static))
        .unwrap();
    assert!(matches!(
        registry.insert(&edge("a", "b", 2.0, Stamp::At(ts(5)))),
        Err(InsertError::KindMismatch { .. })
    ));

    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("a", "b", 2.0, Stamp::At(ts(5))))
        .unwrap();
    assert!(matches!(
        registry.insert(&edge("a", "b", 1.0, Stamp::Static)),
        Err(InsertError::KindMismatch { .. })
    ));
}

#[test]
fn zero_negative_and_extreme_instants_are_ordinary_dynamic_values() {
    // No value is reserved: every instant is a normal dynamic stamp.
    for nanos in [i64::MIN, -SECOND, -1, 0, 1, i64::MAX] {
        let t = ts(nanos);
        let mut registry = Registry::unbounded();
        registry.insert(&edge("a", "b", 1.0, Stamp::At(t))).unwrap();

        assert_eq!(
            registry.lookup("a", "b", t),
            Ok(edge("a", "b", 1.0, Stamp::At(t)))
        );
        assert_eq!(registry.latest_common_time("a", "b"), Ok(Stamp::At(t)));

        // A single-sample dynamic edge cannot serve another instant, so
        // the frame was not classified static.
        let other = ts(if nanos == 0 { 1 } else { 0 });
        let missed = registry.lookup("a", "b", other);
        assert!(
            matches!(
                &missed,
                Err(LookupError::OutOfRange { requested, earliest, latest, .. })
                    if *requested == other && *earliest == t && *latest == t
            ),
            "{missed:?}"
        );
    }
}

#[test]
fn interpolation_crosses_the_epoch() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("a", "b", 0.0, Stamp::At(ts(-10))))
        .unwrap();
    registry
        .insert(&edge("a", "b", 4.0, Stamp::At(ts(30))))
        .unwrap();

    assert_eq!(
        registry.lookup("a", "b", ts(0)),
        Ok(edge("a", "b", 1.0, Stamp::At(ts(0))))
    );
    assert_eq!(
        registry.lookup("a", "b", ts(-5)),
        Ok(edge("a", "b", 0.5, Stamp::At(ts(-5))))
    );
}

#[test]
fn interpolation_spans_the_whole_range_without_overflow() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("a", "b", 0.0, Stamp::At(ts(i64::MIN))))
        .unwrap();
    registry
        .insert(&edge("a", "b", 2.0, Stamp::At(Timestamp::MAX)))
        .unwrap();

    let middle = registry.lookup("a", "b", ts(0)).unwrap();

    approx::assert_abs_diff_eq!(middle.translation().x, 1.0, epsilon = 1e-12);
    assert_eq!(middle.stamp(), Stamp::At(ts(0)));
}

#[test]
fn a_mixed_static_and_dynamic_chain_interpolates() {
    // a -> b static at x = 1; b -> c moves from x = 0 to x = 1 over 10 s.
    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("a", "b", 1.0, Stamp::Static))
        .unwrap();
    registry
        .insert(&edge("b", "c", 0.0, Stamp::At(ts(10 * SECOND))))
        .unwrap();
    registry
        .insert(&edge("b", "c", 1.0, Stamp::At(ts(20 * SECOND))))
        .unwrap();

    let probe = ts(15 * SECOND);
    assert_eq!(
        registry.lookup("a", "c", probe),
        Ok(edge("a", "c", 1.5, Stamp::At(probe)))
    );
}

#[test]
fn the_retention_window_is_measured_in_exact_nanoseconds() {
    let newest = ts(1_700_000_000_000_000_000);
    let max_age = Duration::from_secs(10);
    let boundary = newest.checked_sub(max_age).unwrap();
    let behind = boundary.checked_sub(Duration::from_nanos(1)).unwrap();

    let mut registry = Registry::with_max_age(max_age);
    registry
        .insert(&edge("a", "b", 1.0, Stamp::At(newest)))
        .unwrap();
    registry
        .insert(&edge("a", "b", 0.0, Stamp::At(boundary)))
        .unwrap();

    // At this magnitude one nanosecond is below the resolution of seconds
    // held as `f64`. The window still tells the two instants apart.
    let rejected = registry.insert(&edge("a", "b", 0.0, Stamp::At(behind)));
    assert!(
        matches!(
            &rejected,
            Err(InsertError::Expired { stamp, newest: reported, .. })
                if *stamp == behind && *reported == newest
        ),
        "{rejected:?}"
    );
    // The error leaves the comparison to `Ord`.
    assert!(behind < newest);
    assert_eq!(
        newest.checked_duration_since(behind),
        Some(Duration::new(10, 1))
    );
}

#[test]
fn lookup_errors_carry_exact_instants() {
    let mut registry = Registry::unbounded();
    for nanos in [1_700_000_000_000_000_001, 1_700_000_000_000_000_002] {
        registry
            .insert(&edge("a", "b", 1.0, Stamp::At(ts(nanos))))
            .unwrap();
    }
    let requested = ts(1_700_000_000_000_000_003);

    let error = registry.lookup("a", "b", requested).unwrap_err();

    assert!(
        matches!(
            &error,
            LookupError::OutOfRange { requested: asked, earliest, latest, .. }
                if *asked == requested
                    && *earliest == ts(1_700_000_000_000_000_001)
                    && *latest == ts(1_700_000_000_000_000_002)
        ),
        "{error:?}"
    );
    assert_eq!(
        error.to_string(),
        "frame b covers 1700000000.000000001 to 1700000000.000000002 and not \
         1700000000.000000003"
    );
}

#[test]
fn a_lookup_across_two_instants_keeps_both() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&edge("odom", "base", 1.0, Stamp::At(ts(-SECOND))))
        .unwrap();
    registry
        .insert(&edge("odom", "base", 4.0, Stamp::At(ts(SECOND))))
        .unwrap();

    let across = registry
        .lookup_across("base", ts(SECOND), "base", ts(-SECOND), "odom")
        .unwrap();

    assert_eq!(across.target_time(), ts(SECOND));
    assert_eq!(across.source_time(), ts(-SECOND));
    assert_eq!(across.translation(), Vector3::new(-3.0, 0.0, 0.0));
}
