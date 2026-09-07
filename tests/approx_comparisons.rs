//! Approximate comparisons must tolerate numeric error while keeping frame
//! and time metadata exact. Negative cases also protect the assertions used
//! throughout the geometry suite against comparisons that always succeed.

use approx::{AbsDiffEq, RelativeEq};
use transforms::{
    geometry::{Point, Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

#[test]
fn vector_comparisons_check_every_component_and_honor_both_tolerances() {
    let base = Vector3::new(100.0, 200.0, 300.0);
    assert!(base.abs_diff_eq(&base, 0.0));
    assert!(base.relative_eq(&base, 0.0, 0.0));
    for other in [
        Vector3::new(101.0, 200.0, 300.0),
        Vector3::new(100.0, 201.0, 300.0),
        Vector3::new(100.0, 200.0, 301.0),
    ] {
        assert!(!base.abs_diff_eq(&other, 0.5));
        assert!(base.abs_diff_eq(&other, 1.0));
        assert!(!base.relative_eq(&other, 0.5, 0.001));
        assert!(base.relative_eq(&other, 1.0, 0.0));
        assert!(base.relative_eq(&other, 0.0, 0.01));
    }
}

#[test]
fn quaternion_comparisons_check_every_component_and_honor_both_tolerances() {
    let base = Quaternion::from_wxyz(0.5, 0.5, 0.5, 0.5);
    assert!(base.abs_diff_eq(&base, 0.0));
    assert!(base.relative_eq(&base, 0.0, 0.0));
    for other in [
        Quaternion::from_wxyz(0.625, 0.5, 0.5, 0.5),
        Quaternion::from_wxyz(0.5, 0.625, 0.5, 0.5),
        Quaternion::from_wxyz(0.5, 0.5, 0.625, 0.5),
        Quaternion::from_wxyz(0.5, 0.5, 0.5, 0.625),
    ] {
        assert!(!base.abs_diff_eq(&other, 0.1));
        assert!(base.abs_diff_eq(&other, 0.125));
        assert!(!base.relative_eq(&other, 0.1, 0.1));
        assert!(base.relative_eq(&other, 0.125, 0.0));
        assert!(base.relative_eq(&other, 0.0, 0.25));
    }
}

#[test]
fn transform_comparisons_tolerate_geometry_but_require_exact_metadata() {
    let translation = Vector3::new(100.0, 200.0, 300.0);
    let rotation = Quaternion::from_wxyz(0.5, 0.5, 0.5, 0.5);
    let stamp = Stamp::At(Timestamp::zero());
    let base = Transform::new("a", "b", translation, rotation, stamp).unwrap();
    assert!(base.abs_diff_eq(&base, 0.0));
    assert!(base.relative_eq(&base, 0.0, 0.0));
    for (position, orientation) in [
        (Vector3::new(101.0, 200.0, 300.0), rotation),
        (translation, Quaternion::from_wxyz(0.5, 0.5, 0.5, -0.5)),
    ] {
        let other = Transform::new("a", "b", position, orientation, stamp).unwrap();
        assert!(!base.abs_diff_eq(&other, 0.5));
        assert!(base.abs_diff_eq(&other, 1.0));
        assert!(!base.relative_eq(&other, 0.5, 0.001));
        assert!(base.relative_eq(&other, 1.0, 0.0));
        assert!(base.relative_eq(&other, 0.0, 2.0));
    }
    for (parent, child, timestamp) in [
        ("other", "b", stamp),
        ("a", "other", stamp),
        ("a", "b", Stamp::At(Timestamp::from_nanos(1))),
        ("a", "b", Stamp::Static),
    ] {
        let other = Transform::new(parent, child, translation, rotation, timestamp).unwrap();
        assert!(!base.abs_diff_eq(&other, f64::MAX));
        assert!(!base.relative_eq(&other, f64::MAX, f64::MAX));
    }
}

#[test]
fn point_comparisons_tolerate_geometry_but_require_exact_metadata() {
    let position = Vector3::new(100.0, 200.0, 300.0);
    let orientation = Quaternion::from_wxyz(0.5, 0.5, 0.5, 0.5);
    let time = Timestamp::zero();
    let base = Point::new(position, orientation, time, "a");
    assert!(base.abs_diff_eq(&base, 0.0));
    assert!(base.relative_eq(&base, 0.0, 0.0));
    for (translation, rotation) in [
        (Vector3::new(101.0, 200.0, 300.0), orientation),
        (position, Quaternion::from_wxyz(0.5, 0.5, 0.5, -0.5)),
    ] {
        let other = Point::new(translation, rotation, time, "a");
        assert!(!base.abs_diff_eq(&other, 0.5));
        assert!(base.abs_diff_eq(&other, 1.0));
        assert!(!base.relative_eq(&other, 0.5, 0.001));
        assert!(base.relative_eq(&other, 1.0, 0.0));
        assert!(base.relative_eq(&other, 0.0, 2.0));
    }
    for (timestamp, frame) in [(time, "other"), (Timestamp::from_nanos(1), "a")] {
        let other = Point::new(position, orientation, timestamp, frame);
        assert!(!base.abs_diff_eq(&other, f64::MAX));
        assert!(!base.relative_eq(&other, f64::MAX, f64::MAX));
    }
}
