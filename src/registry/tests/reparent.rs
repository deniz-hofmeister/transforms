//! `Registry::reparent`: the checks, their order, and what a move keeps.

use crate::{InsertError, LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};
use alloc::format;
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

fn disconnected(
    a: &str,
    b: &str,
) -> LookupError {
    LookupError::Disconnected {
        a: a.into(),
        b: b.into(),
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

fn cycle(
    child: &str,
    parent: &str,
) -> InsertError {
    InsertError::Cycle {
        child: child.into(),
        parent: parent.into(),
    }
}

fn kind_mismatch(
    frame: &str,
    incoming: Stamp,
) -> InsertError {
    InsertError::KindMismatch {
        frame: frame.into(),
        incoming,
    }
}

fn parent_unchanged(
    frame: &str,
    parent: &str,
) -> InsertError {
    InsertError::ParentUnchanged {
        frame: frame.into(),
        parent: parent.into(),
    }
}

/// `map <- odom <- base <- lidar`, with `lidar` a static mount.
fn chain() -> Registry {
    let mut registry = Registry::unbounded();
    for nanos in [SEC, 3 * SEC] {
        registry.insert(&sample("map", "odom", nanos, 1.0)).unwrap();
        registry
            .insert(&sample("odom", "base", nanos, 2.0))
            .unwrap();
    }
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    registry
}

#[test]
fn reparent_diagnoses_an_unknown_frame_and_a_root_differently() {
    let mut registry = chain();

    assert_eq!(
        registry.reparent(&sample("map", "ghost", SEC, 1.0)),
        Err(InsertError::UnknownFrame {
            frame: "ghost".into()
        })
    );
    assert_eq!(
        registry.reparent(&sample("world", "map", SEC, 1.0)),
        Err(InsertError::NoParent {
            frame: "map".into()
        })
    );

    // Neither call made a name known.
    assert_eq!(
        position(&registry, "ghost", "ghost", SEC),
        Err(unknown("ghost"))
    );
    assert_eq!(
        position(&registry, "world", "world", SEC),
        Err(unknown("world"))
    );
    // A root gains a parent through an ordinary insert.
    assert_eq!(registry.insert(&sample("world", "map", SEC, 1.0)), Ok(()));
}

#[test]
fn reparent_on_an_empty_registry_reports_an_unknown_frame() {
    let mut registry = Registry::unbounded();
    assert_eq!(
        registry.reparent(&sample("map", "odom", SEC, 1.0)),
        Err(InsertError::UnknownFrame {
            frame: "odom".into()
        })
    );
    assert_eq!(position(&registry, "map", "map", SEC), Err(unknown("map")));
}

#[test]
fn reparent_rejects_an_unchanged_parent_and_keeps_the_history() {
    let mut registry = chain();
    assert_eq!(
        registry.reparent(&sample("map", "odom", 9 * SEC, 9.0)),
        Err(parent_unchanged("odom", "map"))
    );

    // Not an upsert: both stored samples still answer, the seed does not.
    assert_eq!(position(&registry, "map", "odom", SEC), Ok(along_x(1.0)));
    assert_eq!(
        position(&registry, "map", "odom", 9 * SEC),
        Err(out_of_range("odom", 9 * SEC, SEC, 3 * SEC))
    );
}

#[test]
fn reparent_rejects_a_self_edge() {
    let mut registry = chain();
    assert_eq!(
        registry.reparent(&sample("odom", "odom", SEC, 1.0)),
        Err(InsertError::SelfEdge {
            frame: "odom".into()
        })
    );
}

#[test]
fn reparent_rejects_a_move_under_a_descendant() {
    let mut registry = chain();

    // Under its own direct child, and under deeper descendants.
    for parent in ["base", "lidar"] {
        assert_eq!(
            registry.reparent(&sample(parent, "odom", 3 * SEC, 9.0)),
            Err(cycle("odom", parent))
        );
    }

    // The same refusal through drained frames: the tree is the pinned
    // parents, whatever samples they still hold.
    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(
        registry.reparent(&sample("lidar", "odom", 3 * SEC, 9.0)),
        Err(cycle("odom", "lidar"))
    );

    // Every rejection left the tree as it was.
    assert_eq!(
        registry.insert(&sample("lidar", "odom", 3 * SEC, 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "odom".into(),
            pinned: "map".into(),
            offered: "lidar".into(),
        })
    );
}

#[test]
fn reparent_rejects_a_kind_change_in_both_directions() {
    // A dynamic frame, given a static seed.
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    assert_eq!(
        registry.reparent(&fixed("map_b", "odom", 9.0)),
        Err(kind_mismatch("odom", Stamp::Static))
    );
    assert_eq!(position(&registry, "map_a", "odom", SEC), Ok(along_x(1.0)));
    assert_eq!(
        position(&registry, "map_b", "map_b", SEC),
        Err(unknown("map_b"))
    );

    // A static frame, given a dynamic seed.
    let mut registry = Registry::unbounded();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    assert_eq!(
        registry.reparent(&sample("mast", "lidar", 2 * SEC, 9.0)),
        Err(kind_mismatch("lidar", Stamp::At(at(2 * SEC))))
    );
    assert_eq!(
        position(&registry, "base", "lidar", 2 * SEC),
        Ok(along_x(0.5))
    );
    assert_eq!(
        position(&registry, "mast", "mast", SEC),
        Err(unknown("mast"))
    );
}

#[test]
fn competing_reparent_errors_are_reported_in_check_order() {
    let mut registry = chain();
    let before = format!("{registry:?}");

    // An unknown frame, whatever else is wrong with the seed.
    assert_eq!(
        registry.reparent(&fixed("ghost", "ghost", 9.0)),
        Err(InsertError::UnknownFrame {
            frame: "ghost".into()
        })
    );
    // A root, named twice by a seed of either kind.
    assert_eq!(
        registry.reparent(&sample("map", "map", SEC, 9.0)),
        Err(InsertError::NoParent {
            frame: "map".into()
        })
    );
    assert_eq!(
        registry.reparent(&fixed("lidar", "map", 9.0)),
        Err(InsertError::NoParent {
            frame: "map".into()
        })
    );
    // The current parent, with a seed of the other kind.
    assert_eq!(
        registry.reparent(&fixed("map", "odom", 9.0)),
        Err(parent_unchanged("odom", "map"))
    );
    // The frame named twice, with a seed of the other kind. A self-edge is
    // also the shortest cycle.
    assert_eq!(
        registry.reparent(&fixed("odom", "odom", 9.0)),
        Err(InsertError::SelfEdge {
            frame: "odom".into()
        })
    );
    // A descendant as the new parent, with a seed of the other kind.
    assert_eq!(
        registry.reparent(&fixed("lidar", "odom", 9.0)),
        Err(cycle("odom", "lidar"))
    );
    // The other kind alone.
    assert_eq!(
        registry.reparent(&fixed("world", "odom", 9.0)),
        Err(kind_mismatch("odom", Stamp::Static))
    );

    assert_eq!(format!("{registry:?}"), before);
    for name in ["ghost", "world"] {
        assert_eq!(position(&registry, name, name, SEC), Err(unknown(name)));
    }
}

#[test]
fn a_failed_reparent_changes_nothing() {
    let mut registry = Registry::with_max_age(Duration::from_secs(10));
    for nanos in [SEC, 3 * SEC] {
        registry.insert(&sample("map", "odom", nanos, 1.0)).unwrap();
        registry
            .insert(&sample("odom", "base", nanos, 2.0))
            .unwrap();
    }
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    let before = format!("{registry:?}");

    let rejected = [
        sample("map", "ghost", SEC, 9.0),
        sample("world", "map", SEC, 9.0),
        sample("map", "odom", SEC, 9.0),
        sample("odom", "odom", SEC, 9.0),
        sample("lidar", "odom", SEC, 9.0),
        fixed("world", "odom", 9.0),
        sample("world", "lidar", SEC, 9.0),
    ];
    for seed in &rejected {
        assert!(registry.reparent(seed).is_err(), "{seed:?}");
        assert_eq!(format!("{registry:?}"), before);
    }

    assert_eq!(position(&registry, "map", "lidar", SEC), Ok(along_x(3.5)));
    assert_eq!(
        registry.latest_common_time("map", "lidar"),
        Ok(Stamp::At(at(3 * SEC)))
    );
    for name in ["ghost", "world"] {
        assert_eq!(position(&registry, name, name, SEC), Err(unknown(name)));
    }
}

#[test]
fn reparent_drops_the_frames_history_and_collapses_coverage_to_the_seed() {
    let mut registry = Registry::unbounded();
    for (nanos, x) in [(SEC, 1.0), (2 * SEC, 2.0)] {
        registry.insert(&sample("map_a", "odom", nanos, x)).unwrap();
    }

    assert_eq!(
        registry.reparent(&sample("map_b", "odom", 3 * SEC, 5.0)),
        Ok(())
    );

    assert_eq!(
        position(&registry, "map_b", "odom", 3 * SEC),
        Ok(along_x(5.0))
    );
    for dropped in [SEC, 2 * SEC] {
        assert_eq!(
            position(&registry, "map_b", "odom", dropped),
            Err(out_of_range("odom", dropped, 3 * SEC, 3 * SEC))
        );
    }
}

#[test]
fn reparent_keeps_a_dynamic_frame_dynamic() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 2.0))
        .unwrap();

    assert_eq!(
        registry.insert(&fixed("map_b", "odom", 3.0)),
        Err(kind_mismatch("odom", Stamp::Static))
    );
    assert_eq!(
        registry.insert(&sample("map_b", "odom", 3 * SEC, 3.0)),
        Ok(())
    );
    assert_eq!(
        registry.latest_common_time("map_b", "odom"),
        Ok(Stamp::At(at(3 * SEC)))
    );
}

#[test]
fn reparent_moves_a_static_frame_and_keeps_it_static() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    registry.insert(&fixed("base", "mast", 2.0)).unwrap();

    assert_eq!(registry.reparent(&fixed("mast", "lidar", 0.25)), Ok(()));

    // The replaced pose answers every instant, past included.
    for nanos in [i64::MIN, 0, SEC, i64::MAX] {
        assert_eq!(
            position(&registry, "mast", "lidar", nanos),
            Ok(along_x(0.25))
        );
    }
    assert_eq!(position(&registry, "map", "lidar", SEC), Ok(along_x(3.25)));
    assert_eq!(
        registry.latest_common_time("base", "lidar"),
        Ok(Stamp::Static)
    );
    assert_eq!(
        registry.insert(&sample("mast", "lidar", SEC, 9.0)),
        Err(kind_mismatch("lidar", Stamp::At(at(SEC))))
    );
    assert_eq!(registry.insert(&fixed("mast", "lidar", 0.75)), Ok(()));
}

#[test]
fn reparent_moves_a_static_frame_in_a_registry_with_max_age() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry.insert(&sample("map", "base", SEC, 1.0)).unwrap();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    registry.insert(&fixed("base", "mast", 2.0)).unwrap();

    registry.reparent(&fixed("mast", "lidar", 0.25)).unwrap();
    // Dynamic traffic elsewhere advances far past the window.
    registry
        .insert(&sample("map", "base", 500 * SEC, 4.0))
        .unwrap();

    assert_eq!(
        position(&registry, "map", "lidar", 500 * SEC),
        Ok(along_x(6.25))
    );
    assert_eq!(
        registry.insert(&sample("mast", "lidar", 500 * SEC, 9.0)),
        Err(kind_mismatch("lidar", Stamp::At(at(500 * SEC))))
    );
}

#[test]
fn the_seed_is_never_expired_however_far_behind_the_old_newest_stamp() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry
        .insert(&sample("map_a", "odom", 1000 * SEC, 1.0))
        .unwrap();

    // As an insert this stamp is far behind the window ...
    assert_eq!(
        registry.insert(&sample("map_a", "odom", SEC, 2.0)),
        Err(InsertError::Expired {
            frame: "odom".into(),
            stamp: at(SEC),
            newest: at(1000 * SEC),
        })
    );
    // ... and as a seed it replaces the history it would be measured
    // against, so coverage moves backwards.
    assert_eq!(
        registry.reparent(&sample("map_b", "odom", SEC, 2.0)),
        Ok(())
    );
    assert_eq!(position(&registry, "map_b", "odom", SEC), Ok(along_x(2.0)));
    assert_eq!(
        registry.latest_common_time("map_b", "odom"),
        Ok(Stamp::At(at(SEC)))
    );

    // The old newest stamp is gone with the history: the stream continues
    // from the seed.
    assert_eq!(
        registry.insert(&sample("map_b", "odom", SEC + 1, 3.0)),
        Ok(())
    );
}

#[test]
fn retention_still_applies_after_the_move() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("map_b", "odom", 10 * SEC, 2.0))
        .unwrap();

    for (nanos, x) in [(11 * SEC, 3.0), (13 * SEC, 4.0)] {
        registry.insert(&sample("map_b", "odom", nanos, x)).unwrap();
    }

    // The seed and the sample after it are behind the window of the
    // newest insert, and were evicted.
    assert_eq!(
        position(&registry, "map_b", "odom", 10 * SEC),
        Err(out_of_range("odom", 10 * SEC, 13 * SEC, 13 * SEC))
    );
    assert_eq!(
        position(&registry, "map_b", "odom", 13 * SEC),
        Ok(along_x(4.0))
    );
    // The seed's stamp is measured like any stored stamp from here on.
    assert_eq!(
        registry.insert(&sample("map_b", "odom", 10 * SEC, 5.0)),
        Err(InsertError::Expired {
            frame: "odom".into(),
            stamp: at(10 * SEC),
            newest: at(13 * SEC),
        })
    );
}

#[test]
fn a_seed_ahead_of_the_stream_expires_the_samples_behind_it() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("map_b", "odom", 100 * SEC, 2.0))
        .unwrap();

    assert_eq!(
        registry.insert(&sample("map_b", "odom", 2 * SEC, 3.0)),
        Err(InsertError::Expired {
            frame: "odom".into(),
            stamp: at(2 * SEC),
            newest: at(100 * SEC),
        })
    );
    assert_eq!(
        registry.insert(&sample("map_b", "odom", 99 * SEC, 3.0)),
        Ok(())
    );
}

#[test]
fn descendants_keep_their_pins_and_history() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    for (nanos, x) in [(SEC, 2.0), (2 * SEC, 3.0)] {
        registry.insert(&sample("odom", "base", nanos, x)).unwrap();
    }
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();

    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 4.0))
        .unwrap();

    // The subtree answers under the new parent ...
    assert_eq!(
        position(&registry, "map_b", "lidar", 2 * SEC),
        Ok(along_x(7.5))
    );
    // ... the descendants' own history is intact, including the instant
    // the moved edge no longer covers ...
    assert_eq!(position(&registry, "odom", "base", SEC), Ok(along_x(2.0)));
    assert_eq!(position(&registry, "odom", "lidar", SEC), Ok(along_x(2.5)));
    // ... and so are their pins.
    assert_eq!(
        registry.insert(&sample("map_b", "base", 2 * SEC, 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "base".into(),
            pinned: "odom".into(),
            offered: "map_b".into(),
        })
    );
    assert_eq!(
        registry.insert(&sample("base", "lidar", 2 * SEC, 9.0)),
        Err(kind_mismatch("lidar", Stamp::At(at(2 * SEC))))
    );
}

#[test]
fn the_old_parent_becomes_unknown_when_no_other_edge_names_it() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("map_b", "odom", SEC, 2.0))
        .unwrap();

    assert_eq!(
        position(&registry, "map_a", "odom", SEC),
        Err(unknown("map_a"))
    );
    assert_eq!(
        position(&registry, "map_a", "map_a", SEC),
        Err(unknown("map_a"))
    );
    assert!(!registry.remove_frame("map_a"));
}

#[test]
fn the_old_parent_stays_known_while_another_edge_names_it() {
    // As the parent of a sibling.
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    registry.insert(&sample("map", "beacon", SEC, 3.0)).unwrap();
    registry
        .reparent(&sample("world", "odom", SEC, 2.0))
        .unwrap();
    assert_eq!(position(&registry, "map", "beacon", SEC), Ok(along_x(3.0)));
    assert_eq!(
        position(&registry, "map", "odom", SEC),
        Err(disconnected("map", "odom"))
    );

    // As a child of its own parent.
    let mut registry = Registry::unbounded();
    registry.insert(&sample("world", "map", SEC, 4.0)).unwrap();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("world", "odom", SEC, 2.0))
        .unwrap();
    assert_eq!(position(&registry, "world", "map", SEC), Ok(along_x(4.0)));
    assert_eq!(position(&registry, "map", "odom", SEC), Ok(along_x(-2.0)));
}

#[test]
fn an_unknown_new_parent_becomes_known() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    assert_eq!(
        position(&registry, "map_b", "map_b", SEC),
        Err(unknown("map_b"))
    );

    assert_eq!(
        registry.reparent(&sample("map_b", "odom", SEC, 2.0)),
        Ok(())
    );

    assert_eq!(position(&registry, "map_b", "map_b", SEC), Ok(along_x(0.0)));
    assert_eq!(position(&registry, "map_b", "odom", SEC), Ok(along_x(2.0)));
    // It is a root: it has no parent to replace.
    assert_eq!(
        registry.reparent(&sample("world", "map_b", SEC, 1.0)),
        Err(InsertError::NoParent {
            frame: "map_b".into()
        })
    );
}

#[test]
fn reparent_accepts_a_sibling_move() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    registry.insert(&sample("a", "c", SEC, 2.0)).unwrap();

    // The walk upward from `b` reaches the root without meeting `c`.
    assert_eq!(registry.reparent(&sample("b", "c", SEC, 5.0)), Ok(()));
    assert_eq!(position(&registry, "a", "c", SEC), Ok(along_x(6.0)));
}

#[test]
fn reparent_can_move_a_frame_up_to_its_grandparent() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("a", "b", SEC, 1.0)).unwrap();
    registry.insert(&sample("b", "c", SEC, 2.0)).unwrap();

    assert_eq!(registry.reparent(&sample("a", "c", SEC, 5.0)), Ok(()));
    assert_eq!(position(&registry, "a", "c", SEC), Ok(along_x(5.0)));
    // `b` lost its only child and is still `a`'s child.
    assert_eq!(position(&registry, "b", "c", SEC), Ok(along_x(4.0)));
}

#[test]
fn reparent_joins_two_disconnected_trees() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    registry
        .insert(&sample("world", "robot", SEC, 2.0))
        .unwrap();
    registry.insert(&sample("robot", "arm", SEC, 4.0)).unwrap();
    assert_eq!(
        position(&registry, "map", "arm", SEC),
        Err(disconnected("map", "arm"))
    );

    assert_eq!(
        registry.reparent(&sample("odom", "robot", SEC, 8.0)),
        Ok(())
    );

    assert_eq!(position(&registry, "map", "arm", SEC), Ok(along_x(13.0)));
    // The old root was named by no other edge.
    assert_eq!(
        position(&registry, "world", "arm", SEC),
        Err(unknown("world"))
    );
}

#[test]
fn the_old_edges_publisher_is_refused_and_the_new_one_grows_coverage_back() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 2.0))
        .unwrap();

    // The stale publisher is told the frame's current parent.
    assert_eq!(
        registry.insert(&sample("map_a", "odom", 3 * SEC, 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "odom".into(),
            pinned: "map_b".into(),
            offered: "map_a".into(),
        })
    );
    assert_eq!(
        position(&registry, "map_a", "map_a", SEC),
        Err(unknown("map_a"))
    );

    assert_eq!(
        registry.insert(&sample("map_b", "odom", 3 * SEC, 3.0)),
        Ok(())
    );
    assert_eq!(
        position(&registry, "map_b", "odom", 3 * SEC),
        Ok(along_x(3.0))
    );
    assert_eq!(
        position(&registry, "map_b", "odom", SEC),
        Err(out_of_range("odom", SEC, 2 * SEC, 3 * SEC))
    );
}

#[test]
fn cleanup_after_a_move_keeps_the_new_parent_and_kind() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();
    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 2.0))
        .unwrap();

    // A cutoff before the seed keeps it; the cutoff at the seed removes it.
    registry.remove_samples_through(at(2 * SEC - 1));
    assert_eq!(
        position(&registry, "map_b", "odom", 2 * SEC),
        Ok(along_x(2.0))
    );
    registry.remove_samples_through(at(2 * SEC));
    assert_eq!(
        position(&registry, "map_b", "odom", 2 * SEC),
        Err(LookupError::NoData {
            frame: "odom".into()
        })
    );

    assert_eq!(
        registry.insert(&sample("map_a", "odom", 3 * SEC, 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "odom".into(),
            pinned: "map_b".into(),
            offered: "map_a".into(),
        })
    );
    assert_eq!(
        registry.insert(&fixed("map_b", "odom", 9.0)),
        Err(kind_mismatch("odom", Stamp::Static))
    );
    assert_eq!(
        registry.reparent(&sample("map_b", "odom", 3 * SEC, 9.0)),
        Err(parent_unchanged("odom", "map_b"))
    );
}

#[test]
fn the_seed_survives_a_lookup_bit_for_bit() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map_a", "odom", SEC, 1.0)).unwrap();

    let seed = Transform::new(
        "map_b",
        "odom",
        Vector3::new(0.1, -0.2, 0.3),
        Rotation::from_wxyz(0.6, 0.0, 0.0, 0.8).unwrap(),
        Stamp::At(at(2 * SEC)),
    )
    .unwrap();
    registry.reparent(&seed).unwrap();

    assert_eq!(registry.lookup("map_b", "odom", at(2 * SEC)), Ok(seed));
}

#[test]
fn reparent_collapses_latest_common_time_to_the_seed() {
    let mut registry = chain();
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Ok(Stamp::At(at(3 * SEC)))
    );

    // The moved edge serves one instant, and bounds every chain across it.
    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 4.0))
        .unwrap();
    assert_eq!(
        registry.latest_common_time("map_b", "base"),
        Ok(Stamp::At(at(2 * SEC)))
    );

    // A seed past everything the rest of the chain covers leaves no common
    // instant, named on the moved edge.
    registry
        .reparent(&sample("map_c", "odom", 9 * SEC, 4.0))
        .unwrap();
    assert_eq!(
        registry.latest_common_time("map_c", "base"),
        Err(LookupError::NoCommonTime {
            frame: "odom".into(),
            earliest: at(9 * SEC),
            latest: at(9 * SEC),
            latest_possible: at(3 * SEC),
        })
    );
}

#[test]
fn a_cross_time_lookup_over_a_moved_edge_depends_on_the_fixed_frame() {
    let mut registry = Registry::unbounded();
    for (nanos, x) in [(SEC, 1.0), (3 * SEC, 3.0)] {
        registry.insert(&sample("map", "odom", nanos, x)).unwrap();
        registry.insert(&sample("odom", "base", nanos, x)).unwrap();
    }
    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 5.0))
        .unwrap();

    // Fixed frame below the moved edge: served by `odom <- base` alone.
    let across = registry
        .lookup_across("base", at(3 * SEC), "base", at(SEC), "odom")
        .unwrap();
    assert_eq!(across.translation(), Vector3::new(-2.0, 0.0, 0.0));

    // Fixed frame above it: both legs cross the moved edge, which covers
    // only the seed's instant. The target's leg is reported.
    assert_eq!(
        registry.lookup_across("base", at(3 * SEC), "base", at(SEC), "map_b"),
        Err(out_of_range("odom", 3 * SEC, 2 * SEC, 2 * SEC))
    );
}

#[test]
fn successive_moves_each_drop_the_moved_frames_history_only() {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    for (nanos, x) in [(SEC, 2.0), (2 * SEC, 3.0), (3 * SEC, 4.0)] {
        registry.insert(&sample("odom", "base", nanos, x)).unwrap();
    }

    registry
        .reparent(&sample("map_b", "odom", 2 * SEC, 5.0))
        .unwrap();
    registry
        .reparent(&sample("map_c", "odom", 3 * SEC, 6.0))
        .unwrap();

    // The second move dropped the first move's seed ...
    assert_eq!(
        position(&registry, "map_c", "odom", 2 * SEC),
        Err(out_of_range("odom", 2 * SEC, 3 * SEC, 3 * SEC))
    );
    // ... the intermediate root went with its only edge ...
    assert_eq!(
        position(&registry, "map_b", "odom", 2 * SEC),
        Err(unknown("map_b"))
    );
    // ... and the descendant's history survived both moves.
    for (nanos, x) in [(SEC, 2.0), (2 * SEC, 3.0), (3 * SEC, 4.0)] {
        assert_eq!(position(&registry, "odom", "base", nanos), Ok(along_x(x)));
    }

    // Two moved edges on one chain, seeded apart, share no instant.
    registry
        .reparent(&sample("map_c", "base", SEC, 7.0))
        .unwrap();
    registry
        .reparent(&sample("base", "odom", 3 * SEC, 8.0))
        .unwrap();
    assert_eq!(
        registry.latest_common_time("map_c", "odom"),
        Err(LookupError::NoCommonTime {
            frame: "odom".into(),
            earliest: at(3 * SEC),
            latest: at(3 * SEC),
            latest_possible: at(SEC),
        })
    );
}
