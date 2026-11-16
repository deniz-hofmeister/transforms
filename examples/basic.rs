//! Insert transforms, look one up across a static and an interpolated edge,
//! and apply it to a position.
//!
//! Frame tree (parent -> child): map -> base -> camera. A transform with
//! parent `map` and child `base` is `base` expressed in `map`: it maps
//! `base` coordinates into `map`.

use approx::assert_abs_diff_eq;
use core::f64::consts::FRAC_1_SQRT_2;
use transforms::{
    Registry, Transformable,
    errors::RegistryError,
    geometry::{Point, Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

fn main() {
    let at = |seconds: u64| Timestamp::from_nanos(seconds * 1_000_000_000);
    let mut registry = Registry::new();

    // base expressed in map (parent map, child base): yawed 90 degrees,
    // driving along map's x axis. One sample at 1 s, one at 3 s.
    let yaw_90 = Quaternion::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2);
    for (seconds, x) in [(1, 1.0), (3, 3.0)] {
        let map_base = Transform::new(
            "map",
            "base",
            Vector3::new(x, 0.0, 0.0),
            yaw_90,
            Stamp::At(at(seconds)),
        )
        .unwrap();
        registry.add_transform(map_base).unwrap();
    }

    // camera expressed in base (parent base, child camera): a fixed mount.
    // A static transform answers every instant and never expires.
    let base_camera = Transform::static_between(
        "base",
        "camera",
        Vector3::new(0.5, 0.0, 0.0),
        Quaternion::identity(),
    )
    .unwrap();
    registry.add_transform(base_camera).unwrap();

    // camera expressed in map at 2 s: map -> base is interpolated between
    // its two samples, base -> camera is static.
    let map_camera = registry.get_transform("map", "camera", at(2)).unwrap();
    assert_eq!((map_camera.parent(), map_camera.child()), ("map", "camera"));
    assert_eq!(map_camera.timestamp(), Stamp::At(at(2)));
    assert_abs_diff_eq!(
        map_camera.translation(),
        Vector3::new(2.0, 0.5, 0.0),
        epsilon = 1e-12
    );

    // Apply it: a point 1 m ahead of the camera, moved into map coordinates.
    // `Point::transform` checks that the point is in the transform's child
    // frame at the transform's instant, and relabels it on success.
    let mut point = Point::new(
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        at(2),
        "camera",
    );
    point.transform(&map_camera).unwrap();
    assert_eq!(point.frame, "map");
    assert_abs_diff_eq!(point.position, Vector3::new(2.0, 1.5, 0.0), epsilon = 1e-12);

    // The same point cannot be moved twice, or by a transform at another
    // instant: the check refuses instead of returning a wrong position.
    assert!(point.transform(&map_camera).is_err());

    // The opposite direction is the same lookup with the frames swapped.
    let camera_map = registry.get_transform("camera", "map", at(2)).unwrap();
    assert_abs_diff_eq!(
        camera_map.translation(),
        Vector3::new(-0.5, 2.0, 0.0),
        epsilon = 1e-12
    );

    // There is no extrapolation: 4 s lies past the newest map -> base
    // sample, and the error says which frame covers which range.
    match registry.get_transform("map", "camera", at(4)) {
        Err(RegistryError::NotFoundAt { frame, covered, .. }) => {
            assert_eq!(frame, "base");
            assert_eq!(covered, Some((at(1), at(3))));
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }

    println!("camera in map at 2 s: {:?}", map_camera.translation());
}
