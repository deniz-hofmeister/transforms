//! `Registry::insert`: the pins, the check order, upserts and expiry.

use crate::{InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
use alloc::format;
use approx::assert_abs_diff_eq;
use core::time::Duration;

/// One second, in nanoseconds.
const SEC: i64 = 1_000_000_000;

fn at(nanos: i64) -> Timestamp {
    Timestamp::from_nanos(nanos)
}

/// A dynamic sample translated by `x` along the x-axis.
fn sample(
    parent: &str,
    child: &str,
    nanos: i64,
    x: f64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::At(at(nanos)),
    )
    .unwrap()
}

/// A static transform translated by `x` along the x-axis.
fn fixed(
    parent: &str,
    child: &str,
    x: f64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(x, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap()
}

/// The translation of a lookup, or its error.
fn position(
    registry: &Registry,
    target: &str,
    source: &str,
    nanos: i64,
) -> Result<Vector3, LookupError> {
    registry
        .lookup(target, source, at(nanos))
        .map(|transform| transform.translation())
}

fn along_x(x: f64) -> Vector3 {
    Vector3::new(x, 0.0, 0.0)
}

fn unknown(frame: &str) -> LookupError {
    LookupError::UnknownFrame {
        frame: frame.into(),
    }
}

fn out_of_range(
    frame: &str,
    requested: i64,
    earliest: i64,
    latest: i64,
) -> LookupError {
    LookupError::OutOfRange {
        frame: frame.into(),
        requested: at(requested),
        earliest: at(earliest),
        latest: at(latest),
    }
}

fn expired(
    frame: &str,
    stamp: i64,
    newest: i64,
) -> InsertError {
    InsertError::Expired {
        frame: frame.into(),
        stamp: at(stamp),
        newest: at(newest),
    }
}

fn cycle(
    child: &str,
    parent: &str,
) -> InsertError {
    InsertError::Cycle {
        child: child.into(),
        parent: parent.into(),
    }
}

#[test]
fn first_insert_makes_both_names_known_and_stores_the_sample() {
    let mut registry = Registry::unbounded();
    assert_eq!(position(&registry, "map", "map", SEC), Err(unknown("map")));

    let stored = sample("map", "base", SEC, 1.0);
    assert_eq!(registry.insert(&stored), Ok(()));

    assert_eq!(registry.lookup("map", "base", at(SEC)), Ok(stored));
    assert_eq!(position(&registry, "map", "map", SEC), Ok(along_x(0.0)));
    assert_eq!(position(&registry, "base", "base", SEC), Ok(along_x(0.0)));
}

#[test]
fn insert_rejects_cycles() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();

    // Two frames: `b` is under `a`, so `a` cannot go under `b`.
    assert_eq!(
        registry.insert(&sample("b", "a", SEC, 1.0)),
        Err(cycle("a", "b"))
    );
    assert_eq!(position(&registry, "a", "b", SEC), Ok(along_x(1.0)));

    // Three frames: extend the chain, then try to close it.
    registry.insert(&sample("b", "c", SEC, 1.0)).unwrap();
    assert_eq!(
        registry.insert(&sample("c", "a", SEC, 1.0)),
        Err(cycle("a", "c"))
    );
    assert_eq!(position(&registry, "a", "c", SEC), Ok(along_x(2.0)));
}

#[test]
fn insert_rejects_a_cycle_through_a_drained_frame() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    registry.insert(&sample("b", "c", SEC, 1.0)).unwrap();
    registry.remove_samples_through(Timestamp::MAX);

    // The tree is the pinned parents, whatever samples they still hold.
    assert_eq!(
        registry.insert(&sample("c", "a", 2 * SEC, 1.0)),
        Err(cycle("a", "c"))
    );
}

#[test]
fn a_known_root_gaining_its_first_parent_is_checked_as_a_new_edge() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    registry.insert(&fixed("b", "c", 2.0)).unwrap();

    // `a` is known and has no edge. Under its own descendant it would
    // close a cycle, whatever the kind of the offered transform.
    assert_eq!(registry.insert(&fixed("c", "a", 9.0)), Err(cycle("a", "c")));
    assert_eq!(
        registry.insert(&sample("b", "a", SEC, 9.0)),
        Err(cycle("a", "b"))
    );

    // Under a frame outside its subtree it is an ordinary first insert,
    // which pins the parent and the kind.
    assert_eq!(registry.insert(&fixed("world", "a", 4.0)), Ok(()));
    assert_eq!(position(&registry, "world", "c", SEC), Ok(along_x(7.0)));
    assert_eq!(
        registry.insert(&sample("world", "a", SEC, 4.0)),
        Err(InsertError::KindMismatch {
            frame: "a".into(),
            incoming: Stamp::At(at(SEC)),
        })
    );
}

#[test]
fn insert_rejects_a_self_edge_and_makes_no_name_known() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert(&sample("a", "a", SEC, 1.0)),
        Err(InsertError::SelfEdge { frame: "a".into() })
    );
    assert_eq!(position(&registry, "a", "a", SEC), Err(unknown("a")));
}

#[test]
fn a_rejected_self_edge_does_not_bypass_cycle_detection() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.insert(&sample("a", "a", SEC, 1.0)),
        Err(InsertError::SelfEdge { frame: "a".into() })
    );
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();

    // A frame left behind by the rejected insert would let this close the
    // cycle without the check seeing it.
    assert_eq!(
        registry.insert(&sample("b", "a", SEC, -1.0)),
        Err(cycle("a", "b"))
    );
    assert_eq!(position(&registry, "a", "b", SEC), Ok(along_x(1.0)));
}

#[test]
fn insert_rejects_a_second_parent_until_the_frame_is_removed() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&sample("world", "object", SEC, 1.0))
        .unwrap();

    let picked_up = sample("gripper", "object", 2 * SEC, 0.5);
    assert_eq!(
        registry.insert(&picked_up),
        Err(InsertError::ParentMismatch {
            frame: "object".into(),
            pinned: "world".into(),
            offered: "gripper".into(),
        })
    );
    // The offered parent did not become known.
    assert_eq!(
        position(&registry, "gripper", "gripper", SEC),
        Err(unknown("gripper"))
    );

    assert!(registry.remove_frame("object"));
    assert!(!registry.remove_frame("object"));
    assert_eq!(registry.insert(&picked_up), Ok(()));
    assert_eq!(
        position(&registry, "gripper", "object", 2 * SEC),
        Ok(along_x(0.5))
    );
    assert_eq!(
        position(&registry, "world", "object", SEC),
        Err(unknown("world"))
    );
}

#[test]
fn insert_rejects_mixing_static_and_dynamic_in_both_directions() {
    let static_tf = fixed("a", "b", 1.0);
    let dynamic_tf = sample("a", "b", SEC, 2.0);

    let mut registry = Registry::unbounded();
    registry.insert(&static_tf).unwrap();
    assert_eq!(
        registry.insert(&dynamic_tf),
        Err(InsertError::KindMismatch {
            frame: "b".into(),
            incoming: Stamp::At(at(SEC)),
        })
    );
    assert_eq!(position(&registry, "a", "b", SEC), Ok(along_x(1.0)));

    let mut registry = Registry::unbounded();
    registry.insert(&dynamic_tf).unwrap();
    assert_eq!(
        registry.insert(&static_tf),
        Err(InsertError::KindMismatch {
            frame: "b".into(),
            incoming: Stamp::Static,
        })
    );
    assert_eq!(registry.lookup("a", "b", at(SEC)), Ok(dynamic_tf));
    assert_eq!(
        position(&registry, "a", "b", 2 * SEC),
        Err(out_of_range("b", 2 * SEC, SEC, SEC))
    );
}

#[test]
fn competing_errors_on_an_existing_edge_are_reported_in_check_order() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    // Same frame twice, the wrong parent, the wrong kind.
    assert_eq!(
        registry.insert(&fixed("base", "base", 9.0)),
        Err(InsertError::SelfEdge {
            frame: "base".into()
        })
    );
    // The wrong parent and the wrong kind.
    assert_eq!(
        registry.insert(&fixed("other", "base", 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "base".into(),
            pinned: "map".into(),
            offered: "other".into(),
        })
    );
    // The wrong parent and a stamp behind the window.
    assert_eq!(
        registry.insert(&sample("other", "base", SEC, 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "base".into(),
            pinned: "map".into(),
            offered: "other".into(),
        })
    );
    // The wrong kind alone.
    assert_eq!(
        registry.insert(&fixed("map", "base", 9.0)),
        Err(InsertError::KindMismatch {
            frame: "base".into(),
            incoming: Stamp::Static,
        })
    );
    // Expiry is the last check: the sample has the frame's parent and kind.
    assert_eq!(
        registry.insert(&sample("map", "base", SEC, 9.0)),
        Err(expired("base", SEC, 100 * SEC))
    );

    assert_eq!(
        position(&registry, "map", "base", 100 * SEC),
        Ok(along_x(1.0))
    );
    assert_eq!(
        position(&registry, "other", "other", SEC),
        Err(unknown("other"))
    );
}

#[test]
fn a_parent_mismatch_that_would_also_close_a_cycle_reports_the_mismatch() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    registry.insert(&sample("b", "c", SEC, 1.0)).unwrap();

    // `b` has an edge, so the cycle walk does not run for it.
    assert_eq!(
        registry.insert(&sample("c", "b", SEC, 1.0)),
        Err(InsertError::ParentMismatch {
            frame: "b".into(),
            pinned: "a".into(),
            offered: "c".into(),
        })
    );
}

#[test]
fn a_self_edge_on_a_new_frame_is_reported_as_a_self_edge() {
    // A self-edge is also the shortest cycle. On a root that other edges
    // name, both descriptions apply and `SelfEdge` is the one reported.
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    assert_eq!(
        registry.insert(&sample("a", "a", SEC, 1.0)),
        Err(InsertError::SelfEdge { frame: "a".into() })
    );
}

#[test]
fn a_rejected_insert_leaves_the_registry_unchanged() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 50 * SEC, 1.0))
        .unwrap();
    registry
        .insert(&sample("map", "base", 55 * SEC, 2.0))
        .unwrap();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    let before = format!("{registry:?}");

    let rejected = [
        sample("x", "x", 55 * SEC, 9.0),
        sample("x", "base", 55 * SEC, 9.0),
        fixed("map", "base", 9.0),
        sample("base", "lidar", 55 * SEC, 9.0),
        sample("lidar", "map", 55 * SEC, 9.0),
        sample("map", "base", SEC, 9.0),
    ];
    for transform in &rejected {
        assert!(registry.insert(transform).is_err(), "{transform:?}");
        assert_eq!(format!("{registry:?}"), before);
    }

    assert_eq!(position(&registry, "x", "x", SEC), Err(unknown("x")));
    assert_eq!(
        position(&registry, "map", "base", 50 * SEC),
        Ok(along_x(1.0))
    );
    assert_eq!(
        position(&registry, "map", "lidar", 55 * SEC),
        Ok(along_x(2.5))
    );
    assert_eq!(
        registry.latest_common_time("map", "lidar"),
        Ok(Stamp::At(at(55 * SEC)))
    );
}

#[test]
fn a_rejected_transform_stays_with_the_caller() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    let late = sample("map", "base", SEC, 7.0);
    assert_eq!(registry.insert(&late), Err(expired("base", SEC, 100 * SEC)));

    // The rejected sample is still in hand for the recovery.
    assert_eq!(late.translation(), Vector3::new(7.0, 0.0, 0.0));
    assert!(registry.remove_frame("base"));
    assert_eq!(registry.insert(&late), Ok(()));
    assert_eq!(registry.lookup("map", "base", at(SEC)), Ok(late));
}

#[test]
fn a_duplicate_dynamic_stamp_overwrites_the_sample() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", 5 * SEC, 1.0)).unwrap();
    registry.insert(&sample("a", "b", 5 * SEC, 2.0)).unwrap();

    assert_eq!(position(&registry, "a", "b", 5 * SEC), Ok(along_x(2.0)));
    // One sample, not two: the covered range is the single instant.
    assert_eq!(
        position(&registry, "a", "b", 5 * SEC + 1),
        Err(out_of_range("b", 5 * SEC + 1, 5 * SEC, 5 * SEC))
    );
}

#[test]
fn a_static_reinsert_replaces_the_pose() {
    let mut registry = Registry::unbounded();
    registry.insert(&fixed("a", "b", 1.0)).unwrap();
    registry.insert(&fixed("a", "b", 7.0)).unwrap();

    // The replaced pose answers every instant, past included.
    for nanos in [i64::MIN, 0, 3 * SEC, i64::MAX] {
        assert_eq!(position(&registry, "a", "b", nanos), Ok(along_x(7.0)));
    }
    assert_eq!(registry.latest_common_time("a", "b"), Ok(Stamp::Static));
}

#[test]
fn out_of_order_samples_are_stored_and_interpolated() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", 10 * SEC, 10.0)).unwrap();
    registry.insert(&sample("a", "b", 0, 0.0)).unwrap();
    registry
        .insert(&sample("a", "b", -10 * SEC, -10.0))
        .unwrap();

    let middle = position(&registry, "a", "b", 5 * SEC).unwrap();
    assert_abs_diff_eq!(middle.x, 5.0, epsilon = 1e-12);
    assert_eq!(position(&registry, "a", "b", -10 * SEC), Ok(along_x(-10.0)));
    assert_eq!(
        registry.latest_common_time("a", "b"),
        Ok(Stamp::At(at(10 * SEC)))
    );
}

#[test]
fn zero_is_an_ordinary_dynamic_instant() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", 0, 1.0)).unwrap();
    assert_eq!(
        registry.insert(&fixed("a", "b", 1.0)),
        Err(InsertError::KindMismatch {
            frame: "b".into(),
            incoming: Stamp::Static,
        })
    );
    assert_eq!(
        position(&registry, "a", "b", 1),
        Err(out_of_range("b", 1, 0, 0))
    );
}

#[test]
fn with_max_age_evicts_old_samples_on_insert_and_unbounded_keeps_them() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    for nanos in [SEC, 6 * SEC] {
        registry.insert(&sample("a", "b", nanos, 1.0)).unwrap();
    }
    assert_eq!(
        position(&registry, "a", "b", SEC),
        Err(out_of_range("b", SEC, 6 * SEC, 6 * SEC))
    );
    assert_eq!(position(&registry, "a", "b", 6 * SEC), Ok(along_x(1.0)));

    let mut registry = Registry::unbounded();
    for nanos in [SEC, 6 * SEC] {
        registry.insert(&sample("a", "b", nanos, 1.0)).unwrap();
    }
    assert_eq!(position(&registry, "a", "b", SEC), Ok(along_x(1.0)));
    assert_eq!(position(&registry, "a", "b", 6 * SEC), Ok(along_x(1.0)));
}

#[test]
fn an_insert_older_than_the_window_is_expired_and_stores_nothing() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    let late = 90 * SEC - 1;
    assert_eq!(
        registry.insert(&sample("map", "base", late, 2.0)),
        Err(expired("base", late, 100 * SEC))
    );
    assert_eq!(
        position(&registry, "map", "base", late),
        Err(out_of_range("base", late, 100 * SEC, 100 * SEC))
    );
}

#[test]
fn a_sample_exactly_on_the_boundary_is_accepted_and_kept() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    assert_eq!(
        registry.insert(&sample("map", "base", 90 * SEC, 2.0)),
        Ok(())
    );
    assert_eq!(
        position(&registry, "map", "base", 90 * SEC),
        Ok(along_x(2.0))
    );
}

#[test]
fn eviction_keeps_the_boundary_sample_and_drops_the_one_before_it() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 90 * SEC - 1, 1.0))
        .unwrap();
    registry
        .insert(&sample("map", "base", 90 * SEC, 2.0))
        .unwrap();
    registry
        .insert(&sample("map", "base", 100 * SEC, 3.0))
        .unwrap();

    assert_eq!(
        position(&registry, "map", "base", 90 * SEC),
        Ok(along_x(2.0))
    );
    assert_eq!(
        position(&registry, "map", "base", 90 * SEC - 1),
        Err(out_of_range("base", 90 * SEC - 1, 90 * SEC, 100 * SEC))
    );
}

#[test]
fn a_late_sample_inside_the_window_does_not_move_the_newest_stamp_back() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry.insert(&sample("a", "b", 5 * SEC, 1.0)).unwrap();
    // Inside the window: stored beside the newer one.
    registry
        .insert(&sample("a", "b", 4 * SEC + 1, 2.0))
        .unwrap();

    // The window is still measured from 5 s.
    assert_eq!(
        registry.insert(&sample("a", "b", 3 * SEC + 1, 3.0)),
        Err(expired("b", 3 * SEC + 1, 5 * SEC))
    );
    assert_eq!(
        registry.latest_common_time("a", "b"),
        Ok(Stamp::At(at(5 * SEC)))
    );
    assert_eq!(position(&registry, "a", "b", 4 * SEC + 1), Ok(along_x(2.0)));
}

#[test]
fn zero_max_age_keeps_only_the_newest_sample_and_rejects_older_ones() {
    let mut registry = Registry::with_max_age(Duration::ZERO);
    for (nanos, x) in [(SEC, 1.0), (2 * SEC, 2.0), (3 * SEC, 3.0)] {
        registry.insert(&sample("a", "b", nanos, x)).unwrap();
    }

    assert_eq!(position(&registry, "a", "b", 3 * SEC), Ok(along_x(3.0)));
    assert_eq!(
        position(&registry, "a", "b", 2 * SEC),
        Err(out_of_range("b", 2 * SEC, 3 * SEC, 3 * SEC))
    );

    assert_eq!(
        registry.insert(&sample("a", "b", 3 * SEC - 1, 9.0)),
        Err(expired("b", 3 * SEC - 1, 3 * SEC))
    );
    // The newest stamp itself is on the boundary: an upsert.
    assert_eq!(registry.insert(&sample("a", "b", 3 * SEC, 4.0)), Ok(()));
    assert_eq!(position(&registry, "a", "b", 3 * SEC), Ok(along_x(4.0)));
}

#[test]
fn an_unrepresentable_cutoff_rejects_and_evicts_nothing() {
    let mut registry = Registry::with_max_age(Duration::from_nanos(10));
    registry
        .insert(&sample("a", "b", i64::MIN + 5, 1.0))
        .unwrap();

    // The cutoff would lie 5 ns below the earliest instant.
    assert_eq!(registry.insert(&sample("a", "b", i64::MIN, 2.0)), Ok(()));
    assert_eq!(position(&registry, "a", "b", i64::MIN), Ok(along_x(2.0)));
    assert_eq!(
        position(&registry, "a", "b", i64::MIN + 5),
        Ok(along_x(1.0))
    );

    // One step later the cutoff is the earliest instant itself, which is
    // on the boundary and stays.
    registry
        .insert(&sample("a", "b", i64::MIN + 10, 3.0))
        .unwrap();
    assert_eq!(position(&registry, "a", "b", i64::MIN), Ok(along_x(2.0)));

    // And one step after that it is representable and applies.
    registry
        .insert(&sample("a", "b", i64::MIN + 11, 4.0))
        .unwrap();
    assert_eq!(
        position(&registry, "a", "b", i64::MIN),
        Err(out_of_range("b", i64::MIN, i64::MIN + 5, i64::MIN + 11))
    );
    assert_eq!(
        registry.insert(&sample("a", "b", i64::MIN, 5.0)),
        Err(expired("b", i64::MIN, i64::MIN + 11))
    );
}

#[test]
fn a_max_age_longer_than_the_nanosecond_range_still_has_an_exact_cutoff() {
    // 400 years is more nanoseconds than an `i64` holds, and the cutoff
    // behind the latest instant is still representable.
    let max_age = Duration::from_secs(400 * 365 * 24 * 3600);
    assert!(max_age.as_nanos() > i64::MAX.unsigned_abs().into());
    let cutoff = i64::try_from(i128::from(i64::MAX) - 12_614_400_000_000_000_000_i128).unwrap();
    assert_eq!(Timestamp::MAX.checked_sub(max_age), Some(at(cutoff)));

    let mut registry = Registry::with_max_age(max_age);
    registry.insert(&sample("a", "b", cutoff - 1, 1.0)).unwrap();
    registry.insert(&sample("a", "b", cutoff, 2.0)).unwrap();
    registry.insert(&sample("a", "b", i64::MAX, 3.0)).unwrap();

    // Eviction used the exact cutoff ...
    assert_eq!(position(&registry, "a", "b", cutoff), Ok(along_x(2.0)));
    assert_eq!(
        position(&registry, "a", "b", cutoff - 1),
        Err(out_of_range("b", cutoff - 1, cutoff, i64::MAX))
    );
    // ... and so does the insert check.
    assert_eq!(
        registry.insert(&sample("a", "b", cutoff - 1, 4.0)),
        Err(expired("b", cutoff - 1, i64::MAX))
    );
    assert_eq!(registry.insert(&sample("a", "b", cutoff, 5.0)), Ok(()));
    assert_eq!(position(&registry, "a", "b", cutoff), Ok(along_x(5.0)));
}

#[test]
fn an_unbounded_registry_never_expires_a_sample() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", i64::MAX, 1.0)).unwrap();
    assert_eq!(registry.insert(&sample("a", "b", i64::MIN, 2.0)), Ok(()));
    assert_eq!(registry.insert(&sample("a", "b", 0, 3.0)), Ok(()));

    assert_eq!(position(&registry, "a", "b", i64::MIN), Ok(along_x(2.0)));
    assert_eq!(position(&registry, "a", "b", 0), Ok(along_x(3.0)));
    assert_eq!(position(&registry, "a", "b", i64::MAX), Ok(along_x(1.0)));
}

#[test]
fn a_sample_at_the_latest_instant_anchors_the_window_there() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry.insert(&sample("a", "b", 100 * SEC, 1.0)).unwrap();
    registry.insert(&sample("a", "b", i64::MAX, 2.0)).unwrap();

    // The ordinary stream is far behind the window now.
    assert_eq!(
        registry.insert(&sample("a", "b", 101 * SEC, 3.0)),
        Err(expired("b", 101 * SEC, i64::MAX))
    );
    assert_eq!(
        registry.insert(&sample("a", "b", i64::MAX - 10 * SEC - 1, 3.0)),
        Err(expired("b", i64::MAX - 10 * SEC - 1, i64::MAX))
    );

    // Samples inside the window are still accepted, the boundary included.
    assert_eq!(
        registry.insert(&sample("a", "b", i64::MAX - 10 * SEC, 4.0)),
        Ok(())
    );
    assert_eq!(
        registry.insert(&sample("a", "b", i64::MAX - 1, 5.0)),
        Ok(())
    );
    assert_eq!(
        position(&registry, "a", "b", i64::MAX - 10 * SEC),
        Ok(along_x(4.0))
    );
    assert_eq!(
        position(&registry, "a", "b", 100 * SEC),
        Err(out_of_range("b", 100 * SEC, i64::MAX - 10 * SEC, i64::MAX))
    );
}

#[test]
fn a_sample_stamped_ahead_is_accepted_and_the_stream_expires_until_it_catches_up() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    for seconds in 1..=3 {
        registry
            .insert(&sample("map", "base", seconds * SEC, 0.0))
            .unwrap();
    }

    // The sample that moves the window ahead is accepted.
    assert_eq!(
        registry.insert(&sample("map", "base", 100 * SEC, 10.0)),
        Ok(())
    );

    // The stream behind it is rejected sample by sample ...
    for seconds in [4, 50, 89] {
        assert_eq!(
            registry.insert(&sample("map", "base", seconds * SEC, 0.0)),
            Err(expired("base", seconds * SEC, 100 * SEC))
        );
    }
    assert_eq!(
        position(&registry, "map", "base", 3 * SEC),
        Err(out_of_range("base", 3 * SEC, 100 * SEC, 100 * SEC))
    );

    // ... until it reaches the boundary, where it is stored beside the
    // sample ahead and lookups interpolate between the two.
    assert_eq!(
        registry.insert(&sample("map", "base", 90 * SEC, 0.0)),
        Ok(())
    );
    let between = position(&registry, "map", "base", 95 * SEC).unwrap();
    assert_abs_diff_eq!(between.x, 5.0, epsilon = 1e-12);
}

#[test]
fn a_backward_step_smaller_than_max_age_is_stored_without_an_error() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 10.0))
        .unwrap();
    assert_eq!(
        registry.insert(&sample("map", "base", 92 * SEC, 2.0)),
        Ok(())
    );

    let between = position(&registry, "map", "base", 96 * SEC).unwrap();
    assert_abs_diff_eq!(between.x, 6.0, epsilon = 1e-12);
}

#[test]
fn an_upsert_or_a_newer_sample_does_not_recover_an_expired_stream() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    // Replacing the sample ahead keeps its stamp as the newest.
    registry
        .insert(&sample("map", "base", 100 * SEC, 2.0))
        .unwrap();
    assert_eq!(
        registry.insert(&sample("map", "base", 4 * SEC, 0.0)),
        Err(expired("base", 4 * SEC, 100 * SEC))
    );

    // A newer sample moves it further ahead.
    registry
        .insert(&sample("map", "base", 200 * SEC, 3.0))
        .unwrap();
    assert_eq!(
        registry.insert(&sample("map", "base", 4 * SEC, 0.0)),
        Err(expired("base", 4 * SEC, 200 * SEC))
    );
}

#[test]
fn remove_frame_recovers_an_expired_stream() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();
    registry
        .insert(&sample("map", "arm", 100 * SEC, 5.0))
        .unwrap();
    let restarted = sample("map", "base", 4 * SEC, 2.0);
    assert_eq!(
        registry.insert(&restarted),
        Err(expired("base", 4 * SEC, 100 * SEC))
    );

    assert!(registry.remove_frame("base"));
    assert_eq!(registry.insert(&restarted), Ok(()));

    assert_eq!(
        position(&registry, "map", "base", 4 * SEC),
        Ok(along_x(2.0))
    );
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Ok(Stamp::At(at(4 * SEC)))
    );
    // No other frame was touched.
    assert_eq!(
        position(&registry, "map", "arm", 100 * SEC),
        Ok(along_x(5.0))
    );
}

#[test]
fn remove_samples_through_the_newest_stamp_recovers_an_expired_stream() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();
    registry
        .insert(&sample("map", "arm", 99 * SEC, 5.0))
        .unwrap();
    registry
        .insert(&sample("map", "arm", 101 * SEC, 6.0))
        .unwrap();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    let restarted = sample("map", "base", 4 * SEC, 2.0);
    let Err(InsertError::Expired { newest, .. }) = registry.insert(&restarted) else {
        panic!("the restarted stream must be expired");
    };

    // One nanosecond short of the newest stamp keeps it, and the error.
    registry.remove_samples_through(at(newest.as_nanos() - 1));
    assert_eq!(
        registry.insert(&restarted),
        Err(expired("base", 4 * SEC, 100 * SEC))
    );

    registry.remove_samples_through(newest);
    assert_eq!(registry.insert(&restarted), Ok(()));
    assert_eq!(
        position(&registry, "map", "base", 4 * SEC),
        Ok(along_x(2.0))
    );

    // The drained frame kept its parent and kind throughout.
    assert_eq!(
        registry.insert(&sample("world", "base", 4 * SEC, 2.0)),
        Err(InsertError::ParentMismatch {
            frame: "base".into(),
            pinned: "map".into(),
            offered: "world".into(),
        })
    );
    // The cutoff reached every frame: `arm` lost its sample before it,
    // and the static mount is untouched.
    assert_eq!(
        position(&registry, "map", "arm", 99 * SEC),
        Err(out_of_range("arm", 99 * SEC, 101 * SEC, 101 * SEC))
    );
    assert_eq!(position(&registry, "base", "lidar", 0), Ok(along_x(0.5)));
}

#[test]
fn expiry_is_measured_per_frame() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();

    // Another frame's first sample, and its own window.
    assert_eq!(registry.insert(&sample("map", "arm", SEC, 2.0)), Ok(()));
    assert_eq!(registry.insert(&sample("base", "tool", SEC, 3.0)), Ok(()));
    assert_eq!(position(&registry, "map", "arm", SEC), Ok(along_x(2.0)));
    assert_eq!(
        position(&registry, "map", "base", 100 * SEC),
        Ok(along_x(1.0))
    );
}

#[test]
fn a_static_transform_never_expires() {
    let mut registry = Registry::with_max_age(Duration::ZERO);
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    registry
        .insert(&sample("map", "base", i64::MAX, 1.0))
        .unwrap();

    assert_eq!(registry.insert(&fixed("base", "lidar", 0.75)), Ok(()));
    assert_eq!(
        position(&registry, "base", "lidar", i64::MIN),
        Ok(along_x(0.75))
    );
    assert_eq!(
        position(&registry, "map", "lidar", i64::MAX),
        Ok(along_x(1.75))
    );
}
