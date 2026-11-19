//! Insert transforms, look one up across a static and an interpolated edge,
//! apply it to a position, and compose and invert it by hand.
//!
//! Frame tree (parent -> child): map -> base -> camera. A transform with
//! target `map` and source `base` is `base` expressed in `map`: it maps
//! `base` coordinates into `map`. Inserted, it is the edge whose parent is
//! `map` and whose child is `base`.

use approx::assert_abs_diff_eq;
use core::f64::consts::FRAC_1_SQRT_2;
use transforms::{
    LookupError, Registry, Rotation, Stamp, Timestamp, Transform, TransformError, Vector3,
};

/// Compares two vectors component by component.
fn assert_close(
    actual: Vector3,
    expected: Vector3,
) {
    assert_abs_diff_eq!(actual.x, expected.x, epsilon = 1e-12);
    assert_abs_diff_eq!(actual.y, expected.y, epsilon = 1e-12);
    assert_abs_diff_eq!(actual.z, expected.z, epsilon = 1e-12);
}

fn main() {
    let at = |seconds: i64| Timestamp::from_nanos(seconds * 1_000_000_000);
    let mut registry = Registry::unbounded();

    // base expressed in map (target map, source base): yawed 90 degrees,
    // driving along map's x axis. One sample at 1 s, one at 3 s.
    let yaw_90 = Rotation::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2).unwrap();
    for (seconds, x) in [(1, 1.0), (3, 3.0)] {
        let map_base = Transform::new(
            "map",
            "base",
            Vector3::new(x, 0.0, 0.0),
            yaw_90,
            Stamp::At(at(seconds)),
        )
        .unwrap();
        registry.insert(&map_base).unwrap();
    }

    // camera expressed in base (target base, source camera): a fixed mount.
    // A static transform answers every instant and never expires.
    let base_camera = Transform::new(
        "base",
        "camera",
        Vector3::new(0.5, 0.0, 0.0),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap();
    registry.insert(&base_camera).unwrap();

    // camera expressed in map at 2 s: map -> base is interpolated between
    // its two samples, base -> camera is static.
    let map_camera = registry.lookup("map", "camera", at(2)).unwrap();
    assert_eq!(
        (map_camera.target(), map_camera.source()),
        ("map", "camera")
    );
    assert_eq!(map_camera.stamp(), Stamp::At(at(2)));
    assert_close(map_camera.translation(), Vector3::new(2.0, 0.5, 0.0));

    // Apply it: a point 1 m ahead of the camera, moved into map coordinates.
    // The crate does not apply transforms; these three lines are the recipe
    // from the `Transform` documentation. Rotate first, then add the
    // translation. Nothing checks that `p` really is in the transform's
    // source frame at the transform's instant: that is the caller's job.
    let p = Vector3::new(1.0, 0.0, 0.0);
    let r = map_camera.rotation().rotate(p);
    let t = map_camera.translation();
    let p_map = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
    assert_close(p_map, Vector3::new(2.0, 1.5, 0.0));

    // Compose: the same answer built by hand from the two edges. `compose`
    // requires the left operand's source to be the right operand's target,
    // and the static operand adopts the other's stamp.
    let map_base = registry.lookup("map", "base", at(2)).unwrap();
    let composed = map_base.compose(&base_camera).unwrap();
    assert_eq!((composed.target(), composed.source()), ("map", "camera"));
    assert_eq!(composed.stamp(), Stamp::At(at(2)));
    assert_close(composed.translation(), map_camera.translation());

    // The operands the other way round do not chain, and the error says so
    // instead of returning a plausible transform.
    assert!(matches!(
        base_camera.compose(&map_base),
        Err(TransformError::FrameMismatch { expected, found, .. })
            if expected == "camera" && found == "map"
    ));

    // Inverse: the labels swap and the stamp stays. It agrees with the
    // lookup in the opposite direction, which is the same call with the
    // frames swapped.
    let inverted = map_camera.inverse().unwrap();
    let camera_map = registry.lookup("camera", "map", at(2)).unwrap();
    assert_eq!((inverted.target(), inverted.source()), ("camera", "map"));
    assert_eq!(inverted.stamp(), Stamp::At(at(2)));
    assert_close(inverted.translation(), Vector3::new(-0.5, 2.0, 0.0));
    assert_close(camera_map.translation(), inverted.translation());

    // There is no extrapolation: 4 s lies past the newest map -> base
    // sample, and the error says which frame covers which range.
    match registry.lookup("map", "camera", at(4)) {
        Err(LookupError::OutOfRange {
            frame,
            requested,
            earliest,
            latest,
            ..
        }) => {
            assert_eq!(frame, "base");
            assert_eq!((requested, earliest, latest), (at(4), at(1), at(3)));
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }

    println!("camera in map at 2 s: {:?}", map_camera.translation());
    println!("point 1 m ahead of the camera, in map: {p_map:?}");
}
