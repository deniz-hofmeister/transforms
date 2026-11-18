//! `Registry::remove_frame` and `Registry::remove_samples_through`.

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

fn no_data(frame: &str) -> LookupError {
    LookupError::NoData {
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

/// `map <- odom <- base`, all dynamic at 1 s.
fn chain() -> Registry {
    let mut registry = Registry::unbounded();
    registry.insert(&sample("map", "odom", SEC, 1.0)).unwrap();
    registry.insert(&sample("odom", "base", SEC, 2.0)).unwrap();
    registry
}

#[test]
fn remove_frame_returns_true_iff_the_child_had_an_edge() {
    let mut registry = chain();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    let before = format!("{registry:?}");

    // A root, which is also a name known only as a parent; an unknown
    // name; the empty name.
    for name in ["map", "ghost", ""] {
        assert!(!registry.remove_frame(name), "{name:?}");
        assert_eq!(format!("{registry:?}"), before);
    }

    // A dynamic leaf-side edge, a static edge, an internal edge.
    assert!(registry.remove_frame("lidar"));
    assert!(!registry.remove_frame("lidar"));
    assert!(registry.remove_frame("odom"));
    assert!(!registry.remove_frame("odom"));
    assert!(registry.remove_frame("base"));
    assert!(!registry.remove_frame("base"));
}

#[test]
fn remove_frame_on_an_empty_registry_returns_false() {
    let mut registry = Registry::unbounded();
    assert!(!registry.remove_frame("map"));
    assert!(!registry.remove_frame(""));
}

#[test]
fn children_of_a_removed_frame_keep_their_pins_and_stay_connected_below() {
    let mut registry = chain();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();

    assert!(registry.remove_frame("odom"));

    // The removed frame is the root of its former subtree.
    assert_eq!(position(&registry, "odom", "lidar", SEC), Ok(along_x(2.5)));
    assert_eq!(position(&registry, "odom", "odom", SEC), Ok(along_x(0.0)));
    assert_eq!(
        registry.insert(&sample("map", "base", SEC, 9.0)),
        Err(InsertError::ParentMismatch {
            frame: "base".into(),
            pinned: "odom".into(),
            offered: "map".into(),
        })
    );
    assert_eq!(
        registry.insert(&sample("base", "lidar", SEC, 9.0)),
        Err(InsertError::KindMismatch {
            frame: "lidar".into(),
            incoming: Stamp::At(at(SEC)),
        })
    );
    // Its former parent was named by no other edge.
    assert_eq!(position(&registry, "map", "base", SEC), Err(unknown("map")));
    // It has no parent to replace, and it is not removable again.
    assert_eq!(
        registry.reparent(&sample("world", "odom", SEC, 1.0)),
        Err(InsertError::NoParent {
            frame: "odom".into()
        })
    );
    assert!(!registry.remove_frame("odom"));
}

#[test]
fn a_removed_frames_parent_stays_known_while_another_edge_names_it() {
    let mut registry = chain();
    registry.insert(&sample("map", "beacon", SEC, 4.0)).unwrap();

    assert!(registry.remove_frame("odom"));

    assert_eq!(position(&registry, "map", "beacon", SEC), Ok(along_x(4.0)));
    // Both are known, and nothing connects them any more.
    assert_eq!(
        position(&registry, "map", "base", SEC),
        Err(disconnected("map", "base"))
    );
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Err(disconnected("map", "base"))
    );
}

#[test]
fn removing_a_leaf_makes_it_unknown() {
    let mut registry = chain();
    assert!(registry.remove_frame("base"));

    assert_eq!(
        position(&registry, "base", "base", SEC),
        Err(unknown("base"))
    );
    assert_eq!(
        position(&registry, "map", "base", SEC),
        Err(unknown("base"))
    );
    // `odom` is still `map`'s child.
    assert_eq!(position(&registry, "map", "odom", SEC), Ok(along_x(1.0)));
}

#[test]
fn readding_a_removed_frame_under_a_new_parent_reconnects_its_subtree() {
    let mut registry = chain();
    registry.insert(&sample("world", "map", SEC, 4.0)).unwrap();

    assert!(registry.remove_frame("odom"));
    registry.insert(&sample("world", "odom", SEC, 8.0)).unwrap();

    // `base` was never touched and answers through its untouched history.
    assert_eq!(position(&registry, "world", "base", SEC), Ok(along_x(10.0)));
    assert_eq!(position(&registry, "map", "base", SEC), Ok(along_x(6.0)));
}

#[test]
fn remove_frame_releases_the_parent_and_the_kind() {
    let mut registry = chain();
    assert!(registry.remove_frame("base"));

    // Another parent and the other kind: a first insert again.
    assert_eq!(registry.insert(&fixed("map", "base", 3.0)), Ok(()));
    assert_eq!(
        position(&registry, "map", "base", 7 * SEC),
        Ok(along_x(3.0))
    );
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Ok(Stamp::Static)
    );

    // And back to dynamic under the first parent.
    assert!(registry.remove_frame("base"));
    assert_eq!(
        registry.insert(&sample("odom", "base", 2 * SEC, 5.0)),
        Ok(())
    );
    assert_eq!(
        position(&registry, "odom", "base", SEC),
        Err(out_of_range("base", SEC, 2 * SEC, 2 * SEC))
    );
}

#[test]
fn a_removed_frame_is_a_root_above_its_children_for_the_cycle_check() {
    let mut registry = chain();
    assert!(registry.remove_frame("odom"));

    // `odom` is a root above `base` now, and `map` is unknown: the edge
    // that was a cycle before the removal is still one from `base`, and
    // an ordinary first insert from `map`.
    assert_eq!(
        registry.insert(&sample("base", "odom", SEC, 1.0)),
        Err(InsertError::Cycle {
            child: "odom".into(),
            parent: "base".into(),
        })
    );
    assert_eq!(registry.insert(&sample("base", "map", SEC, 1.0)), Ok(()));
    assert_eq!(position(&registry, "odom", "map", SEC), Ok(along_x(3.0)));
}

#[test]
fn remove_frame_discards_the_newest_stamp() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();
    assert_eq!(
        registry.insert(&sample("map", "base", SEC, 2.0)),
        Err(InsertError::Expired {
            frame: "base".into(),
            stamp: at(SEC),
            newest: at(100 * SEC),
        })
    );

    assert!(registry.remove_frame("base"));
    assert_eq!(registry.insert(&sample("map", "base", SEC, 2.0)), Ok(()));
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Ok(Stamp::At(at(SEC)))
    );
}

#[test]
fn remove_frame_discards_every_sample_of_the_edge() {
    let mut registry = Registry::unbounded();
    for nanos in [SEC, 2 * SEC, 3 * SEC] {
        registry.insert(&sample("map", "base", nanos, 1.0)).unwrap();
    }
    registry.insert(&sample("map", "arm", SEC, 1.0)).unwrap();

    assert!(registry.remove_frame("base"));
    registry
        .insert(&sample("map", "base", 2 * SEC, 7.0))
        .unwrap();

    assert_eq!(
        position(&registry, "map", "base", SEC),
        Err(out_of_range("base", SEC, 2 * SEC, 2 * SEC))
    );
    assert_eq!(
        position(&registry, "map", "base", 2 * SEC),
        Ok(along_x(7.0))
    );
}

#[test]
fn the_cutoff_is_inclusive() {
    let mut registry = Registry::unbounded();
    for (nanos, x) in [(10, 1.0), (20, 2.0), (30, 3.0)] {
        registry.insert(&sample("map", "base", nanos, x)).unwrap();
    }

    // Between two samples: the older one goes.
    registry.remove_samples_through(at(19));
    assert_eq!(
        position(&registry, "map", "base", 10),
        Err(out_of_range("base", 10, 20, 30))
    );
    assert_eq!(position(&registry, "map", "base", 20), Ok(along_x(2.0)));

    // On a sample: that sample goes too.
    registry.remove_samples_through(at(20));
    assert_eq!(
        position(&registry, "map", "base", 20),
        Err(out_of_range("base", 20, 30, 30))
    );
    assert_eq!(position(&registry, "map", "base", 30), Ok(along_x(3.0)));

    // A cutoff before every sample removes nothing.
    registry.remove_samples_through(at(29));
    assert_eq!(position(&registry, "map", "base", 30), Ok(along_x(3.0)));

    // On the newest sample: the frame is drained.
    registry.remove_samples_through(at(30));
    assert_eq!(position(&registry, "map", "base", 30), Err(no_data("base")));
}

#[test]
fn the_cutoff_works_at_both_ends_of_the_time_range() {
    let mut registry = Registry::unbounded();
    for (nanos, x) in [
        (i64::MIN, 1.0),
        (i64::MIN + 1, 2.0),
        (i64::MAX - 1, 3.0),
        (i64::MAX, 4.0),
    ] {
        registry.insert(&sample("map", "base", nanos, x)).unwrap();
    }

    // The earliest instant removes the sample stored there and no other.
    registry.remove_samples_through(at(i64::MIN));
    assert_eq!(
        position(&registry, "map", "base", i64::MIN),
        Err(out_of_range("base", i64::MIN, i64::MIN + 1, i64::MAX))
    );
    assert_eq!(
        position(&registry, "map", "base", i64::MIN + 1),
        Ok(along_x(2.0))
    );

    // One nanosecond before the latest instant keeps the sample at it.
    registry.remove_samples_through(at(i64::MAX - 1));
    assert_eq!(
        position(&registry, "map", "base", i64::MAX - 1),
        Err(out_of_range("base", i64::MAX - 1, i64::MAX, i64::MAX))
    );
    assert_eq!(
        position(&registry, "map", "base", i64::MAX),
        Ok(along_x(4.0))
    );
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Ok(Stamp::At(Timestamp::MAX))
    );

    // The latest instant removes that one too.
    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(
        position(&registry, "map", "base", i64::MAX),
        Err(no_data("base"))
    );
    assert_eq!(
        registry.latest_common_time("map", "base"),
        Err(no_data("base"))
    );
}

#[test]
fn a_sample_at_the_latest_instant_is_removable_under_max_age() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry
        .insert(&sample("map", "base", i64::MAX, 1.0))
        .unwrap();
    assert_eq!(
        registry.insert(&sample("map", "base", SEC, 2.0)),
        Err(InsertError::Expired {
            frame: "base".into(),
            stamp: at(SEC),
            newest: Timestamp::MAX,
        })
    );

    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(registry.insert(&sample("map", "base", SEC, 2.0)), Ok(()));
    assert_eq!(position(&registry, "map", "base", SEC), Ok(along_x(2.0)));
}

#[test]
fn static_transforms_are_untouched_by_any_cutoff() {
    let mut registry = Registry::unbounded();
    registry.insert(&fixed("base", "lidar", 0.5)).unwrap();
    registry
        .insert(&sample("map", "base", 5 * SEC, 1.0))
        .unwrap();

    for cutoff in [at(i64::MIN), at(5 * SEC), Timestamp::MAX] {
        registry.remove_samples_through(cutoff);
        assert_eq!(
            position(&registry, "base", "lidar", 9 * SEC),
            Ok(along_x(0.5))
        );
        assert_eq!(
            registry.latest_common_time("base", "lidar"),
            Ok(Stamp::Static)
        );
    }
    // The lookup result carries the requested instant.
    assert_eq!(
        registry
            .lookup("base", "lidar", at(9 * SEC))
            .unwrap()
            .stamp(),
        Stamp::At(at(9 * SEC))
    );
    // The dynamic edge above it is drained.
    assert_eq!(
        position(&registry, "map", "lidar", 5 * SEC),
        Err(no_data("base"))
    );
}

#[test]
fn a_drained_frame_keeps_its_parent() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&sample("world", "object", SEC, 1.0))
        .unwrap();
    registry.remove_samples_through(at(3 * SEC));

    let picked_up = sample("gripper", "object", 3 * SEC, 0.5);
    assert_eq!(
        registry.insert(&picked_up),
        Err(InsertError::ParentMismatch {
            frame: "object".into(),
            pinned: "world".into(),
            offered: "gripper".into(),
        })
    );
    assert_eq!(
        registry.reparent(&sample("world", "object", 3 * SEC, 0.5)),
        Err(InsertError::ParentUnchanged {
            frame: "object".into(),
            parent: "world".into(),
        })
    );

    // `remove_frame` is the release, drained or not.
    assert!(registry.remove_frame("object"));
    assert_eq!(registry.insert(&picked_up), Ok(()));
    assert_eq!(
        position(&registry, "gripper", "object", 3 * SEC),
        Ok(along_x(0.5))
    );
}

#[test]
fn a_drained_frame_keeps_its_kind() {
    let mut registry = Registry::unbounded();
    registry
        .insert(&sample("world", "object", SEC, 1.0))
        .unwrap();
    registry.remove_samples_through(at(3 * SEC));

    assert_eq!(
        registry.insert(&fixed("world", "object", 0.5)),
        Err(InsertError::KindMismatch {
            frame: "object".into(),
            incoming: Stamp::Static,
        })
    );
    assert_eq!(
        registry.reparent(&fixed("gripper", "object", 0.5)),
        Err(InsertError::KindMismatch {
            frame: "object".into(),
            incoming: Stamp::Static,
        })
    );
    // Dynamic inserts keep working.
    assert_eq!(
        registry.insert(&sample("world", "object", 2 * SEC, 2.0)),
        Ok(())
    );
    assert_eq!(
        position(&registry, "world", "object", 2 * SEC),
        Ok(along_x(2.0))
    );
}

#[test]
fn a_drained_frame_stays_known_and_reports_no_data() {
    let mut registry = chain();
    registry.remove_samples_through(at(3 * SEC));

    // Known and empty, which is not a timing problem and not a typo.
    assert_eq!(
        position(&registry, "odom", "base", 3 * SEC),
        Err(no_data("base"))
    );
    assert_eq!(
        position(&registry, "map", "base", 3 * SEC),
        Err(no_data("base"))
    );
    assert_eq!(
        registry.latest_common_time("map", "odom"),
        Err(no_data("odom"))
    );
    for name in ["map", "odom", "base"] {
        assert_eq!(position(&registry, name, name, SEC), Ok(along_x(0.0)));
    }
    assert!(registry.remove_frame("base"));
}

#[test]
fn a_drained_frame_has_no_hidden_newest_stamp() {
    let mut registry = Registry::with_max_age(Duration::from_secs(1));
    registry
        .insert(&sample("map", "base", 100 * SEC, 1.0))
        .unwrap();
    registry.remove_samples_through(at(100 * SEC));

    // A stream restarted far behind the removed stamp is kept.
    assert_eq!(registry.insert(&sample("map", "base", 0, 2.0)), Ok(()));
    assert_eq!(position(&registry, "map", "base", 0), Ok(along_x(2.0)));

    // The window is measured from the restarted stream.
    assert_eq!(
        registry.insert(&sample("map", "base", 2 * SEC, 3.0)),
        Ok(())
    );
    assert_eq!(
        registry.insert(&sample("map", "base", SEC - 1, 4.0)),
        Err(InsertError::Expired {
            frame: "base".into(),
            stamp: at(SEC - 1),
            newest: at(2 * SEC),
        })
    );
}

#[test]
fn the_cutoff_applies_to_every_frame() {
    let mut registry = Registry::unbounded();
    for (parent, child) in [("map", "odom"), ("odom", "base"), ("world", "robot")] {
        for (nanos, x) in [(SEC, 1.0), (3 * SEC, 3.0)] {
            registry.insert(&sample(parent, child, nanos, x)).unwrap();
        }
    }
    registry
        .insert(&sample("world", "beacon", SEC, 1.0))
        .unwrap();

    registry.remove_samples_through(at(2 * SEC));

    for (parent, child) in [("map", "odom"), ("odom", "base"), ("world", "robot")] {
        assert_eq!(
            position(&registry, parent, child, SEC),
            Err(out_of_range(child, SEC, 3 * SEC, 3 * SEC))
        );
        assert_eq!(
            position(&registry, parent, child, 3 * SEC),
            Ok(along_x(3.0))
        );
    }
    assert_eq!(
        position(&registry, "world", "beacon", SEC),
        Err(no_data("beacon"))
    );
}

#[test]
fn a_cutoff_on_an_empty_registry_does_nothing() {
    let mut registry = Registry::unbounded();
    registry.remove_samples_through(Timestamp::MAX);
    assert_eq!(
        format!("{registry:?}"),
        format!("{:?}", Registry::unbounded())
    );
}
