//! `History`: what one edge stores, and how it is sampled.

use super::lookup::{at, bits, tilted, yaw};
use crate::{
    Rotation, Stamp, Timestamp, Vector3,
    geometry::Isometry,
    registry::history::{Gap, History, SampleError},
};
use alloc::vec::Vec;
use approx::assert_abs_diff_eq;
use core::time::Duration;

/// An isometry at `x` on the x axis, without rotation.
fn shifted(x: f64) -> Isometry {
    Isometry::new(Vector3::new(x, 0.0, 0.0), Rotation::IDENTITY).unwrap()
}

/// A dynamic history holding `shifted(x)` at each `(nanos, x)`, stored in
/// the given order without a retention window.
fn dynamic(samples: &[(i64, f64)]) -> History {
    let [(first_nanos, first_x), rest @ ..] = samples else {
        panic!("a dynamic history is seeded with a sample");
    };
    let mut history = History::seeded(shifted(*first_x), Stamp::At(at(*first_nanos)));
    for (nanos, x) in rest {
        assert!(history.store(shifted(*x), Stamp::At(at(*nanos)), None));
    }
    history
}

/// The stored instants of a dynamic history, oldest first.
fn stored_nanos(history: &History) -> Vec<i64> {
    let History::Dynamic(samples) = history else {
        panic!("expected a dynamic history, found {history:?}");
    };
    samples.keys().map(|stamp| stamp.as_nanos()).collect()
}

fn isometry_bits(isometry: Isometry) -> [u64; 7] {
    bits(isometry.translation(), isometry.rotation())
}

fn outside(
    earliest: i64,
    latest: i64,
) -> Gap {
    Gap::Outside {
        earliest: at(earliest),
        latest: at(latest),
    }
}

#[test]
fn seeded_with_a_static_stamp_is_a_static_history() {
    let history = History::seeded(shifted(1.0), Stamp::Static);

    assert!(history.is_static());
    assert!(matches!(history, History::Static(pose) if pose == shifted(1.0)));
    assert_eq!(history.range(), None);
}

#[test]
fn seeded_with_an_instant_is_a_dynamic_history_holding_that_sample() {
    let history = History::seeded(shifted(1.0), Stamp::At(at(7)));

    assert!(!history.is_static());
    assert_eq!(stored_nanos(&history), [7]);
    assert_eq!(history.range(), Some((at(7), at(7))));
    assert_eq!(history.sample(at(7)), Ok(shifted(1.0)));
}

#[test]
fn zero_is_an_ordinary_dynamic_instant() {
    let history = History::seeded(shifted(1.0), Stamp::At(at(0)));

    assert!(!history.is_static());
    assert_eq!(history.sample(at(0)), Ok(shifted(1.0)));
    assert_eq!(history.sample(at(1)), Err(SampleError::Gap(outside(0, 0))));
}

#[test]
fn static_store_replaces_the_pose() {
    let mut history = History::seeded(shifted(1.0), Stamp::Static);

    assert!(history.store(shifted(2.0), Stamp::Static, None));

    assert!(history.is_static());
    assert_eq!(history.sample(at(0)), Ok(shifted(2.0)));
}

#[test]
fn static_store_ignores_the_retention_window() {
    let mut history = History::seeded(shifted(1.0), Stamp::Static);

    assert!(history.store(shifted(2.0), Stamp::Static, Some(Duration::ZERO)));

    assert_eq!(history.sample(Timestamp::MAX), Ok(shifted(2.0)));
}

#[test]
fn dynamic_store_adds_samples_in_any_order() {
    let history = dynamic(&[(3_000, 3.0), (1_000, 1.0), (7_000, 7.0)]);

    assert_eq!(stored_nanos(&history), [1_000, 3_000, 7_000]);
    assert_eq!(history.range(), Some((at(1_000), at(7_000))));
}

#[test]
fn dynamic_store_at_a_stored_instant_replaces_that_sample() {
    let mut history = dynamic(&[(10, 1.0), (20, 2.0)]);

    assert!(history.store(shifted(9.0), Stamp::At(at(10)), None));

    assert_eq!(stored_nanos(&history), [10, 20]);
    assert_eq!(history.sample(at(10)), Ok(shifted(9.0)));
    assert_eq!(history.sample(at(20)), Ok(shifted(2.0)));
}

#[test]
fn store_of_the_other_kind_returns_false_and_stores_nothing() {
    let mut fixed = History::seeded(shifted(1.0), Stamp::Static);
    assert!(!fixed.store(shifted(2.0), Stamp::At(at(5)), None));
    assert!(fixed.is_static());
    assert_eq!(fixed.sample(at(5)), Ok(shifted(1.0)));

    let mut moving = dynamic(&[(10, 1.0)]);
    assert!(!moving.store(shifted(2.0), Stamp::Static, None));
    assert!(!moving.is_static());
    assert_eq!(stored_nanos(&moving), [10]);
    assert_eq!(moving.sample(at(10)), Ok(shifted(1.0)));
}

#[test]
fn store_under_max_age_evicts_samples_older_than_the_window() {
    let max_age = Some(Duration::from_secs(10));
    let mut history = History::seeded(shifted(0.0), Stamp::At(at(20_000_000_000)));
    for nanos in [25_000_000_000, 31_000_000_000, 36_000_000_000] {
        assert!(history.store(shifted(1.0), Stamp::At(at(nanos)), max_age));
    }

    assert_eq!(stored_nanos(&history), [31_000_000_000, 36_000_000_000]);
}

#[test]
fn eviction_keeps_a_sample_exactly_max_age_old() {
    let max_age = Some(Duration::from_secs(10));
    let mut history = History::seeded(shifted(1.0), Stamp::At(at(100_000_000_000)));

    assert!(history.store(shifted(2.0), Stamp::At(at(110_000_000_000)), max_age));

    assert_eq!(stored_nanos(&history), [100_000_000_000, 110_000_000_000]);
    assert_eq!(history.sample(at(100_000_000_000)), Ok(shifted(1.0)));
}

#[test]
fn eviction_removes_a_sample_one_nanosecond_past_max_age() {
    let max_age = Some(Duration::from_secs(10));
    let mut history = History::seeded(shifted(1.0), Stamp::At(at(100_000_000_000)));

    assert!(history.store(shifted(2.0), Stamp::At(at(110_000_000_001)), max_age));

    assert_eq!(stored_nanos(&history), [110_000_000_001]);
    assert_eq!(history.sample(at(110_000_000_001)), Ok(shifted(2.0)));
}

#[test]
fn zero_max_age_keeps_only_the_newest_sample() {
    let max_age = Some(Duration::ZERO);
    let mut history = History::seeded(shifted(1.0), Stamp::At(at(1_000)));

    assert!(history.store(shifted(2.0), Stamp::At(at(2_000)), max_age));
    assert_eq!(stored_nanos(&history), [2_000]);

    // An upsert of the newest sample is on the boundary and stays.
    assert!(history.store(shifted(3.0), Stamp::At(at(2_000)), max_age));
    assert_eq!(stored_nanos(&history), [2_000]);
    assert_eq!(history.sample(at(2_000)), Ok(shifted(3.0)));
}

#[test]
fn eviction_measures_against_the_newest_stored_instant_and_not_the_last_insert() {
    let max_age = Some(Duration::from_secs(1));
    let mut history = History::seeded(shifted(5.0), Stamp::At(at(5_000_000_000)));

    // A late sample outside the window is stored and evicted by the same
    // call. `Registry::insert` refuses it before it gets here.
    assert!(history.store(shifted(1.0), Stamp::At(at(1_000_000_000)), max_age));

    assert_eq!(stored_nanos(&history), [5_000_000_000]);
    assert_eq!(history.sample(at(5_000_000_000)), Ok(shifted(5.0)));
}

#[test]
fn late_sample_inside_the_window_is_kept() {
    let max_age = Some(Duration::from_secs(1));
    let mut history = History::seeded(shifted(5.0), Stamp::At(at(5_000_000_000)));

    assert!(history.store(shifted(4.0), Stamp::At(at(4_000_000_000)), max_age));

    assert_eq!(stored_nanos(&history), [4_000_000_000, 5_000_000_000]);
}

#[test]
fn store_without_max_age_never_evicts() {
    let mut history = History::seeded(shifted(0.0), Stamp::At(at(0)));

    assert!(history.store(shifted(1.0), Stamp::At(Timestamp::MAX), None));
    assert!(history.store(shifted(2.0), Stamp::At(at(i64::MIN)), None));

    assert_eq!(stored_nanos(&history), [i64::MIN, 0, i64::MAX]);
}

#[test]
fn window_that_reaches_below_the_earliest_instant_evicts_nothing() {
    let max_age = Some(Duration::from_nanos(10));
    let mut history = History::seeded(shifted(0.0), Stamp::At(at(i64::MIN)));

    assert!(history.store(shifted(1.0), Stamp::At(at(i64::MIN + 5)), max_age));
    assert_eq!(stored_nanos(&history), [i64::MIN, i64::MIN + 5]);

    // The boundary becomes representable at `MIN + 10` and is inclusive.
    assert!(history.store(shifted(2.0), Stamp::At(at(i64::MIN + 10)), max_age));
    assert_eq!(
        stored_nanos(&history),
        [i64::MIN, i64::MIN + 5, i64::MIN + 10]
    );
    assert!(history.store(shifted(3.0), Stamp::At(at(i64::MIN + 11)), max_age));
    assert_eq!(
        stored_nanos(&history),
        [i64::MIN + 5, i64::MIN + 10, i64::MIN + 11]
    );
}

#[test]
fn range_is_none_for_static_and_for_drained_histories() {
    assert_eq!(History::seeded(shifted(1.0), Stamp::Static).range(), None);

    let mut history = dynamic(&[(1_000, 1.0)]);
    assert_eq!(history.range(), Some((at(1_000), at(1_000))));
    history.remove_through(at(1_000));
    assert_eq!(history.range(), None);
}

#[test]
fn remove_through_is_inclusive() {
    let mut history = dynamic(&[(10, 1.0), (20, 2.0), (30, 3.0)]);

    history.remove_through(at(9));
    assert_eq!(stored_nanos(&history), [10, 20, 30]);

    history.remove_through(at(20));
    assert_eq!(stored_nanos(&history), [30]);
    assert_eq!(history.range(), Some((at(30), at(30))));

    history.remove_through(at(29));
    assert_eq!(stored_nanos(&history), [30]);

    history.remove_through(at(30));
    assert_eq!(stored_nanos(&history), Vec::<i64>::new());
}

#[test]
fn remove_through_the_latest_instant_clears_everything() {
    let mut history = dynamic(&[(i64::MIN, 0.0), (0, 1.0), (i64::MAX, 2.0)]);

    history.remove_through(Timestamp::MAX);

    assert_eq!(stored_nanos(&history), Vec::<i64>::new());
    assert_eq!(history.range(), None);
    assert_eq!(history.gap(at(0)), Some(Gap::Empty));
}

#[test]
fn remove_through_leaves_a_static_pose_untouched() {
    let mut history = History::seeded(shifted(1.0), Stamp::Static);

    history.remove_through(Timestamp::MAX);

    assert!(history.is_static());
    assert_eq!(history.sample(at(0)), Ok(shifted(1.0)));
    assert_eq!(history.gap(at(0)), None);
}

#[test]
fn drained_history_keeps_its_kind() {
    let mut history = dynamic(&[(10, 1.0)]);
    history.remove_through(Timestamp::MAX);

    assert!(!history.is_static());
    assert!(!history.store(shifted(2.0), Stamp::Static, None));
    assert_eq!(stored_nanos(&history), Vec::<i64>::new());

    assert!(history.store(shifted(3.0), Stamp::At(at(5)), None));
    assert_eq!(stored_nanos(&history), [5]);
}

#[test]
fn drained_history_has_no_newest_instant_to_evict_against() {
    let max_age = Some(Duration::from_secs(1));
    let mut history = History::seeded(shifted(1.0), Stamp::At(at(100_000_000_000)));
    history.remove_through(at(100_000_000_000));

    // Far older than the drained sample: the stream restarts here.
    assert!(history.store(shifted(2.0), Stamp::At(at(1_000_000_000)), max_age));

    assert_eq!(stored_nanos(&history), [1_000_000_000]);
    assert_eq!(history.sample(at(1_000_000_000)), Ok(shifted(2.0)));
}

#[test]
fn gap_is_none_for_a_static_history_at_every_instant() {
    let history = History::seeded(shifted(1.0), Stamp::Static);

    for instant in [at(i64::MIN), at(0), Timestamp::MAX] {
        assert_eq!(history.gap(instant), None);
    }
}

#[test]
fn gap_is_empty_for_a_drained_history() {
    let mut history = dynamic(&[(10, 1.0)]);
    history.remove_through(at(10));

    for nanos in [9, 10, 11] {
        assert_eq!(history.gap(at(nanos)), Some(Gap::Empty));
    }
}

#[test]
fn gap_outside_the_range_carries_the_range() {
    let history = dynamic(&[(10, 1.0), (15, 1.5), (20, 2.0)]);

    assert_eq!(history.gap(at(9)), Some(outside(10, 20)));
    assert_eq!(history.gap(at(21)), Some(outside(10, 20)));
    assert_eq!(history.gap(at(i64::MIN)), Some(outside(10, 20)));
    assert_eq!(history.gap(Timestamp::MAX), Some(outside(10, 20)));
}

#[test]
fn gap_is_none_inside_the_range_ends_included() {
    let history = dynamic(&[(10, 1.0), (20, 2.0)]);

    for nanos in [10, 11, 15, 19, 20] {
        assert_eq!(history.gap(at(nanos)), None);
    }
}

#[test]
fn sample_of_a_static_history_ignores_the_instant() {
    let pose = Isometry::new(Vector3::new(1.0, 2.0, 3.0), tilted()).unwrap();
    let history = History::seeded(pose, Stamp::Static);

    for instant in [at(i64::MIN), at(0), at(1_000_000_000), Timestamp::MAX] {
        let sampled = history.sample(instant).unwrap();
        assert_eq!(isometry_bits(sampled), isometry_bits(pose));
    }
}

#[test]
fn sample_at_a_stored_instant_is_the_stored_isometry_bit_for_bit() {
    let stored = Isometry::new(Vector3::new(1.1, -2.2, 3.3), tilted()).unwrap();
    let before = Isometry::new(Vector3::new(0.7, 0.1, -0.4), yaw(0.3)).unwrap();
    let after = Isometry::new(Vector3::new(-5.0, 0.2, 0.9), yaw(-1.1)).unwrap();
    let mut history = History::seeded(before, Stamp::At(at(1_000)));
    assert!(history.store(stored, Stamp::At(at(2_000)), None));
    assert!(history.store(after, Stamp::At(at(3_000)), None));

    assert_eq!(
        isometry_bits(history.sample(at(2_000)).unwrap()),
        isometry_bits(stored)
    );
    assert_eq!(
        isometry_bits(history.sample(at(1_000)).unwrap()),
        isometry_bits(before)
    );
    assert_eq!(
        isometry_bits(history.sample(at(3_000)).unwrap()),
        isometry_bits(after)
    );
}

#[test]
fn sample_between_two_samples_is_their_midpoint_at_half_time() {
    let quarter_turn = yaw(core::f64::consts::FRAC_PI_2);
    let start = Isometry::new(Vector3::new(1.0, 0.0, 0.0), Rotation::IDENTITY).unwrap();
    let end = Isometry::new(Vector3::new(0.0, 1.0, 0.0), quarter_turn).unwrap();
    let mut history = History::seeded(start, Stamp::At(at(1_000_000_000)));
    assert!(history.store(end, Stamp::At(at(2_000_000_000)), None));

    let sampled = history.sample(at(1_500_000_000)).unwrap();

    assert_eq!(sampled.translation(), Vector3::new(0.5, 0.5, 0.0));
    let eighth_turn = yaw(core::f64::consts::FRAC_PI_4);
    assert_abs_diff_eq!(sampled.rotation().w(), eighth_turn.w(), epsilon = 1e-15);
    assert_abs_diff_eq!(sampled.rotation().z(), eighth_turn.z(), epsilon = 1e-15);
    assert_eq!(sampled, start.interpolate(end, 0.5).unwrap());
}

#[test]
fn sample_interpolates_between_the_two_samples_that_bracket_the_instant() {
    let history = dynamic(&[(0, 0.0), (10, 10.0), (20, 110.0)]);

    for (nanos, x) in [(5, 5.0), (15, 60.0), (1, 1.0), (19, 100.0)] {
        assert_eq!(history.sample(at(nanos)), Ok(shifted(x)), "at {nanos}");
    }
}

#[test]
fn sample_ratio_is_computed_from_integer_nanoseconds() {
    // Near this instant a second count held in an `f64` resolves no finer
    // than 256 ns, so a ratio computed from seconds could not be one third.
    let from = 1_700_000_000_000_000_001;
    let start = Isometry::new(Vector3::new(1.0, 10.0, -3.0), Rotation::IDENTITY).unwrap();
    let end = Isometry::new(Vector3::new(2.0, 40.0, 7.0), yaw(1.0)).unwrap();
    let mut history = History::seeded(start, Stamp::At(at(from)));
    assert!(history.store(end, Stamp::At(at(from + 3)), None));

    assert_eq!(
        isometry_bits(history.sample(at(from + 1)).unwrap()),
        isometry_bits(start.interpolate(end, 1.0 / 3.0).unwrap())
    );
    assert_eq!(
        isometry_bits(history.sample(at(from + 2)).unwrap()),
        isometry_bits(start.interpolate(end, 2.0 / 3.0).unwrap())
    );
}

#[test]
fn sample_ratio_spans_the_whole_time_axis_without_overflow() {
    let mut history = History::seeded(shifted(0.0), Stamp::At(at(i64::MIN)));
    assert!(history.store(shifted(2.0), Stamp::At(Timestamp::MAX), None));

    // The span is 2^64 - 1 ns, more than an `i64` difference holds.
    let sampled = history.sample(at(0)).unwrap();
    assert_abs_diff_eq!(sampled.translation().x, 1.0, epsilon = 1e-15);
}

#[test]
fn sample_does_not_extrapolate_on_either_side() {
    let history = dynamic(&[(10, 1.0), (20, 2.0)]);

    assert_eq!(
        history.sample(at(9)),
        Err(SampleError::Gap(outside(10, 20)))
    );
    assert_eq!(
        history.sample(at(21)),
        Err(SampleError::Gap(outside(10, 20)))
    );
    assert_eq!(
        history.sample(at(i64::MIN)),
        Err(SampleError::Gap(outside(10, 20)))
    );
    assert_eq!(
        history.sample(Timestamp::MAX),
        Err(SampleError::Gap(outside(10, 20)))
    );
}

#[test]
fn sample_of_a_single_sample_history_serves_only_that_instant() {
    let history = dynamic(&[(1_000_000_000, 1.0)]);

    assert_eq!(history.sample(at(1_000_000_000)), Ok(shifted(1.0)));
    assert_eq!(
        history.sample(at(999_999_999)),
        Err(SampleError::Gap(outside(1_000_000_000, 1_000_000_000)))
    );
    assert_eq!(
        history.sample(at(1_000_000_001)),
        Err(SampleError::Gap(outside(1_000_000_000, 1_000_000_000)))
    );
}

#[test]
fn sample_of_a_drained_history_is_an_empty_gap() {
    let mut history = dynamic(&[(10, 1.0), (20, 2.0)]);
    history.remove_through(at(20));

    for nanos in [9, 10, 15, 20, 21] {
        assert_eq!(history.sample(at(nanos)), Err(SampleError::Gap(Gap::Empty)));
    }
}

#[test]
fn sample_and_gap_agree_on_coverage() {
    let history = dynamic(&[(10, 1.0), (20, 2.0), (40, 4.0)]);

    for nanos in 5..=45 {
        match history.gap(at(nanos)) {
            Some(gap) => assert_eq!(history.sample(at(nanos)), Err(SampleError::Gap(gap))),
            None => assert!(history.sample(at(nanos)).is_ok(), "at {nanos}"),
        }
    }
}

#[test]
fn sample_after_removal_serves_only_what_is_left() {
    let mut history = dynamic(&[(1_000_000_000, 1.0), (3_000_000_000, 3.0)]);

    history.remove_through(at(1_999_999_999));

    assert_eq!(
        history.sample(at(1_000_000_000)),
        Err(SampleError::Gap(outside(3_000_000_000, 3_000_000_000)))
    );
    assert_eq!(history.sample(at(3_000_000_000)), Ok(shifted(3.0)));
}

#[test]
fn sample_between_translations_at_the_largest_finite_value_stays_finite() {
    // A blend of two finite translations is a convex combination: its
    // factors never sum past one by enough to round the result to infinity.
    let mut history = History::seeded(shifted(f64::MAX), Stamp::At(at(0)));
    assert!(history.store(shifted(f64::MAX), Stamp::At(at(3_000)), None));

    for nanos in [1, 1_000, 1_500, 2_000, 2_999] {
        let sampled = history.sample(at(nanos)).unwrap();
        assert!(sampled.translation().x.is_finite(), "at {nanos}");
    }

    let mut opposed = History::seeded(shifted(f64::MAX), Stamp::At(at(0)));
    assert!(opposed.store(shifted(-f64::MAX), Stamp::At(at(2)), None));
    assert_eq!(opposed.sample(at(1)), Ok(shifted(0.0)));
}
