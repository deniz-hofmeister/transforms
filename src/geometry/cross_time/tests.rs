//! Unit tests.

use super::CrossTimeTransform;
use crate::{Rotation, Timestamp, Vector3, geometry::Isometry};
use alloc::format;

fn quarter_turn_z() -> Rotation {
    let half = core::f64::consts::FRAC_1_SQRT_2;
    Rotation::from_wxyz(half, 0.0, 0.0, half).unwrap()
}

fn sample() -> CrossTimeTransform {
    CrossTimeTransform::new(
        "map".into(),
        Timestamp::from_nanos(2_000_000_000),
        "base".into(),
        Timestamp::from_nanos(-1_000_000_000),
        Isometry::new(Vector3::new(1.0, 2.0, 3.0), quarter_turn_z()).unwrap(),
    )
}

/// `sample()` with one part replaced.
fn variant(
    target: &str,
    target_time: i64,
    source: &str,
    source_time: i64,
    translation: Vector3,
    rotation: Rotation,
) -> CrossTimeTransform {
    CrossTimeTransform::new(
        target.into(),
        Timestamp::from_nanos(target_time),
        source.into(),
        Timestamp::from_nanos(source_time),
        Isometry::new(translation, rotation).unwrap(),
    )
}

#[test]
fn accessors_return_the_frames_instants_and_geometry_it_was_built_from() {
    let across = sample();

    assert_eq!(across.target(), "map");
    assert_eq!(across.target_time(), Timestamp::from_nanos(2_000_000_000));
    assert_eq!(across.source(), "base");
    assert_eq!(across.source_time(), Timestamp::from_nanos(-1_000_000_000));
    assert_eq!(across.translation(), Vector3::new(1.0, 2.0, 3.0));
    assert_eq!(across.rotation(), quarter_turn_z());
}

#[test]
fn the_two_frames_and_the_two_instants_are_not_swapped() {
    // Every label distinct, so a crossed accessor cannot pass.
    let across = variant(
        "a",
        1,
        "b",
        2,
        Vector3::new(0.0, 0.0, 0.0),
        Rotation::IDENTITY,
    );

    assert_eq!((across.target(), across.target_time().as_nanos()), ("a", 1));
    assert_eq!((across.source(), across.source_time().as_nanos()), ("b", 2));
}

#[test]
fn a_clone_equals_its_original() {
    let across = sample();
    let clone = across.clone();

    assert_eq!(clone, across);
    assert_eq!(clone.target(), across.target());
    assert_eq!(clone.source_time(), across.source_time());
}

#[test]
fn equality_is_exact_over_every_part() {
    let t = Vector3::new(1.0, 2.0, 3.0);
    let r = quarter_turn_z();
    let (t1, t0) = (2_000_000_000, -1_000_000_000);

    assert_eq!(variant("map", t1, "base", t0, t, r), sample());

    assert_ne!(variant("odom", t1, "base", t0, t, r), sample());
    assert_ne!(variant("map", t1 + 1, "base", t0, t, r), sample());
    assert_ne!(variant("map", t1, "lidar", t0, t, r), sample());
    assert_ne!(variant("map", t1, "base", t0 + 1, t, r), sample());
    assert_ne!(
        variant("map", t1, "base", t0, Vector3::new(1.0, 2.0, 3.5), r),
        sample()
    );
    assert_ne!(
        variant("map", t1, "base", t0, t, Rotation::IDENTITY),
        sample()
    );
    // The frames and the instants are positional.
    assert_ne!(variant("base", t1, "map", t0, t, r), sample());
    assert_ne!(variant("map", t0, "base", t1, t, r), sample());
}

#[test]
fn debug_names_the_type_the_frames_and_the_instants() {
    let text = format!("{:?}", sample());

    assert!(text.starts_with("CrossTimeTransform {"), "{text}");
    for part in [
        "target: \"map\"",
        "target_time: Timestamp(2000000000)",
        "source: \"base\"",
        "source_time: Timestamp(-1000000000)",
        "translation: Vector3 { x: 1.0, y: 2.0, z: 3.0 }",
        "Rotation {",
    ] {
        assert!(text.contains(part), "{part} missing from {text}");
    }
}
