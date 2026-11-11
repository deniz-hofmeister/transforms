//! Insert, look up, interpolate, and apply a transform to a position.
//!
//! Frame tree (parent -> child): map -> base (dynamic), base -> lidar (static).
//!
//! Runs unchanged with and without the crate's `std` feature.

use approx::assert_abs_diff_eq;
use core::f64::consts::FRAC_1_SQRT_2;
use transforms::{
    Registry, Transform,
    errors::RegistryError,
    geometry::{Quaternion, Vector3},
    time::{Stamp, Timestamp},
};

/// Maps a position given in `frame` at `at` into `tf.parent()`.
///
/// The crate has no apply method: the formula is `R p + t`, and the two
/// assertions are what `Point::transform` used to check in 2.x. Without
/// them a swapped lookup or a lookup at another instant applies silently.
/// An orientation takes `tf.rotation() * q`; a free vector (a velocity, a
/// normal) takes only the rotation.
fn position_in_parent(
    tf: &Transform,
    frame: &str,
    at: Timestamp,
    position: Vector3,
) -> Vector3 {
    assert_eq!(tf.child(), frame);
    assert!(tf.timestamp().at().is_none_or(|stamp| stamp == at));
    tf.rotation().rotate_vector(position) + tf.translation()
}

fn main() {
    let t1 = Timestamp::from_nanos(1_000_000_000);
    let t2 = Timestamp::from_nanos(2_000_000_000);
    let midway = Timestamp::from_nanos(1_500_000_000);

    let mut registry: Registry = Registry::new();

    // A fixed mount: valid at every instant, yawed 90 degrees.
    let yaw_90 = Quaternion::from_wxyz(FRAC_1_SQRT_2, 0.0, 0.0, FRAC_1_SQRT_2);
    registry
        .add_transform(
            Transform::static_between("base", "lidar", Vector3::new(0.2, 0.0, 0.1), yaw_90)
                .unwrap(),
        )
        .unwrap();

    // The robot moves from x = 1 to x = 3 between t1 and t2.
    for (stamp, x) in [(t1, 1.0), (t2, 3.0)] {
        registry
            .add_transform(
                Transform::new(
                    "map",
                    "base",
                    Vector3::new(x, 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(stamp),
                )
                .unwrap(),
            )
            .unwrap();
    }

    // A stored instant on a single edge comes back exactly.
    let stored = registry.get_transform("map", "base", t1).unwrap();
    assert_eq!(stored.translation(), Vector3::new(1.0, 0.0, 0.0));

    // Between samples the edge is interpolated; the static mount chains in.
    let tf = registry.get_transform("map", "lidar", midway).unwrap();
    assert_eq!((tf.parent(), tf.child()), ("map", "lidar"));
    assert_eq!(tf.timestamp(), Stamp::At(midway));
    assert_abs_diff_eq!(
        tf.translation(),
        Vector3::new(2.2, 0.0, 0.1),
        epsilon = 1e-12
    );

    // A lidar return 1 m ahead of the sensor, expressed in the map.
    let hit = position_in_parent(&tf, "lidar", midway, Vector3::new(1.0, 0.0, 0.0));
    assert_abs_diff_eq!(hit, Vector3::new(2.2, 1.0, 0.1), epsilon = 1e-12);

    // There is no extrapolation: past the newest sample the lookup fails
    // and says which frame covers what.
    let t3 = Timestamp::from_nanos(3_000_000_000);
    match registry.get_transform("map", "lidar", t3) {
        Err(RegistryError::NotFoundAt {
            frame,
            requested,
            covered,
            ..
        }) => {
            assert_eq!(frame, "base");
            assert_eq!(requested, t3);
            assert_eq!(covered, Some((t1, t2)));
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }

    println!("basic: ok");
}
