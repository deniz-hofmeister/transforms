//! Public behavior that internal storage and traversal changes must preserve.

use core::time::Duration;
use transforms::{
    Registry,
    errors::{RegistryError, TransformError},
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

fn sample(
    parent: &str,
    child: &str,
    nanos: u64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(-0.0, 2.0, -3.0),
        Quaternion::from_wxyz(1.0 - 9e-7, 0.0, 0.0, 0.0),
        Stamp::At(Timestamp::from_nanos(nanos)),
    )
    .unwrap()
}

#[test]
fn disconnected_lookup_preserves_sampling_failure_precedence() {
    let mut registry = Registry::new();
    registry.add_transform(sample("a", "b", 1)).unwrap();
    registry.add_transform(sample("x", "y", 1)).unwrap();
    assert!(matches!(
        registry.get_transform("b", "y", Timestamp::from_nanos(2)),
        Err(RegistryError::NotFoundAt { frame, .. }) if frame == "b"
    ));
    assert!(matches!(
        registry.latest_common_time("b", "y"),
        Err(RegistryError::Disconnected { .. })
    ));
}

#[test]
fn failed_lookup_preserves_failure_above_common_ancestor() {
    let mut registry = Registry::new();
    for (parent, child, nanos) in [
        ("root", "common", 1),
        ("common", "target", 2),
        ("common", "source", 1),
    ] {
        registry
            .add_transform(sample(parent, child, nanos))
            .unwrap();
    }
    assert!(matches!(
        registry.get_transform("target", "source", Timestamp::from_nanos(2)),
        Err(RegistryError::NotFoundAt { frame, .. }) if frame == "common"
    ));
}

#[test]
fn exact_sample_preserves_signed_zero_and_accepted_rotation_norm() {
    let stored = sample("parent", "child", 7);
    let mut registry = Registry::new();
    registry.add_transform(stored.clone()).unwrap();
    let found = registry
        .get_transform("parent", "child", Timestamp::from_nanos(7))
        .unwrap();
    assert_eq!(found, stored);
    assert_eq!(found.translation().x.to_bits(), (-0.0_f64).to_bits());
    assert_eq!(found.rotation().w.to_bits(), (1.0_f64 - 9e-7).to_bits());
}

#[test]
fn attaching_an_existing_root_changes_descendant_depth() {
    let mut registry = Registry::new();
    registry.add_transform(sample("root", "leaf", 1)).unwrap();
    registry.add_transform(sample("world", "root", 1)).unwrap();
    let found = registry
        .get_transform("world", "leaf", Timestamp::from_nanos(1))
        .unwrap();
    assert_eq!(found.parent(), "world");
    assert_eq!(found.child(), "leaf");
    assert!(registry.remove_frame("root"));
    assert!(matches!(
        registry.get_transform("world", "leaf", Timestamp::from_nanos(1)),
        Err(RegistryError::UnknownFrame(frame)) if frame == "world"
    ));
    assert!(
        registry
            .get_transform("root", "leaf", Timestamp::from_nanos(1))
            .is_ok()
    );
}

#[test]
fn unbounded_default_interpolates_across_long_gaps() {
    let mut registry = Registry::default();
    registry
        .add_transform(sample("parent", "child", 0))
        .unwrap();
    registry
        .add_transform(sample("parent", "child", 30_000_000_000))
        .unwrap();
    assert!(
        registry
            .get_transform("parent", "child", Timestamp::from_nanos(15_000_000_000))
            .is_ok()
    );
}

#[test]
fn right_identity_retains_v2_composition_error() {
    let identity = Transform::new(
        "child",
        "child",
        Vector3::zero(),
        Quaternion::identity(),
        Stamp::At(Timestamp::from_nanos(1)),
    )
    .unwrap();
    assert!(matches!(
        sample("parent", "child", 1) * identity,
        Err(TransformError::SameFrameMultiplication { frame }) if frame == "child"
    ));
}

#[test]
fn cross_time_result_keeps_only_target_stamp_and_remains_insertable() {
    let mut registry = Registry::new();
    registry
        .add_transform(sample("world", "source", 1))
        .unwrap();
    registry
        .add_transform(sample("world", "target", 2))
        .unwrap();
    let found = registry
        .get_transform_at(
            "target",
            Timestamp::from_nanos(2),
            "source",
            Timestamp::from_nanos(1),
            "world",
        )
        .unwrap();
    assert_eq!(found.timestamp(), Stamp::At(Timestamp::from_nanos(2)));
    let mut receiver = Registry::new();
    receiver.add_transform(found).unwrap();
    assert!(
        receiver
            .get_transform("target", "source", Timestamp::from_nanos(2))
            .is_ok()
    );
}

#[test]
fn timestamp_operators_remain_usable_in_typed_downstream_code() {
    let added: Result<Timestamp, _> = Timestamp::zero() + Duration::from_nanos(3);
    let subtracted: Result<Timestamp, _> = added.unwrap() - Duration::from_nanos(1);
    let elapsed: Result<Duration, _> = subtracted.unwrap() - Timestamp::zero();
    assert_eq!(elapsed.unwrap(), Duration::from_nanos(2));
}

#[test]
fn constant_dynamic_reference_is_accepted_as_a_fixed_frame() {
    let mut registry = Registry::new();
    for (parent, child, nanos) in [
        ("root", "reference", 1),
        ("root", "reference", 2),
        ("reference", "source", 1),
        ("reference", "target", 2),
    ] {
        registry
            .add_transform(sample(parent, child, nanos))
            .unwrap();
    }
    assert!(
        registry
            .get_transform_at(
                "target",
                Timestamp::from_nanos(2),
                "source",
                Timestamp::from_nanos(1),
                "reference",
            )
            .is_ok()
    );
}
