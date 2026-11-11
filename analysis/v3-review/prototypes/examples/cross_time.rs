//! Express a detection made at one instant in a frame at a later instant.
//!
//! `get_transform_at` changes the coordinate reference across time through
//! a frame the caller declares stationary; it does not predict motion.
//!
//! Frame tree (parent -> child): map -> conveyor -> object, map -> camera.
//!
//! Runs unchanged with and without the crate's `std` feature.

use transforms::{
    Registry, Transform,
    errors::RegistryError,
    geometry::{CrossTimeTransform, Quaternion, Vector3},
    time::{Stamp, Timestamp},
};

/// Maps a position given in `frame` at `at` into the target frame at the
/// target time.
///
/// A cross-time result has no apply method, as a `Transform` has none: the
/// formula is `R p + t`. The assertions use the four label accessors, which
/// is what they are for: a result handed on from its call site still says
/// which frame and instant it takes and which it yields.
fn position_in_target(
    tf: &CrossTimeTransform,
    frame: &str,
    at: Timestamp,
    position: Vector3,
) -> Vector3 {
    assert_eq!(tf.source_frame(), frame);
    assert_eq!(tf.source_time(), at);
    tf.rotation().rotate_vector(position) + tf.translation()
}

fn sample(
    parent: &str,
    child: &str,
    translation: Vector3,
    stamp: Timestamp,
) -> Transform {
    Transform::new(
        parent,
        child,
        translation,
        Quaternion::identity(),
        Stamp::At(stamp),
    )
    .unwrap()
}

fn main() {
    let t1 = Timestamp::from_nanos(1_000_000_000);
    let t2 = Timestamp::from_nanos(2_000_000_000);

    let mut registry: Registry = Registry::new();
    registry
        .add_transforms([
            // The conveyor moves from x = 1 to x = 3 in the map.
            sample("map", "conveyor", Vector3::new(1.0, 0.0, 0.0), t1),
            sample("map", "conveyor", Vector3::new(3.0, 0.0, 0.0), t2),
            // The object rides on it at y = 0.5.
            sample("conveyor", "object", Vector3::new(0.0, 0.5, 0.0), t1),
            sample("conveyor", "object", Vector3::new(0.0, 0.5, 0.0), t2),
            // The camera does not move.
            sample("map", "camera", Vector3::new(0.0, 2.0, 0.0), t1),
            sample("map", "camera", Vector3::new(0.0, 2.0, 0.0), t2),
        ])
        .unwrap();

    // "Where was the object at t1, seen from the camera at t2?" The map is
    // the frame that stood still between the two instants.
    let seen = registry
        .get_transform_at("camera", t2, "object", t1, "map")
        .unwrap();
    assert_eq!(seen.target_frame(), "camera");
    assert_eq!(seen.target_time(), t2);
    assert_eq!(seen.source_frame(), "object");
    assert_eq!(seen.source_time(), t1);
    assert_eq!(seen.translation(), Vector3::new(1.0, -1.5, 0.0));

    // A point 0.1 m above the object's origin at t1, in the camera at t2.
    let above = position_in_target(&seen, "object", t1, Vector3::new(0.0, 0.0, 0.1));
    assert_eq!(above, Vector3::new(1.0, -1.5, 0.1));

    // "How far did the object travel?" Its t2 origin in its own t1 frame.
    let travelled = registry
        .get_transform_at("object", t1, "object", t2, "map")
        .unwrap();
    assert_eq!(travelled.translation(), Vector3::new(2.0, 0.0, 0.0));

    // The result holds at two instants, so it is not a `Transform`, and the
    // operations that would treat it as holding at one do not exist. Each of
    // these lines is a compile error:
    //
    //     registry.add_transform(travelled);
    //         error[E0308]: mismatched types: expected `Transform`, found
    //         `CrossTimeTransform`
    //     let composed = seen * travelled;
    //         error[E0369]: cannot multiply `CrossTimeTransform` by
    //         `CrossTimeTransform`
    //     let back = seen.inverse();
    //         error[E0599]: no method named `inverse` found for struct
    //         `CrossTimeTransform`
    //
    // In 2.x all three compiled, and the insert was accepted whenever the
    // target was the source's parent, replacing that edge's honest sample
    // at the target time. Rebuilding a `Transform` from the accessors to
    // get the operations back defeats the type: do not.

    // Topology outranks sampling across both legs. `camera` cannot serve
    // t = 9 s, but the target is in no tree at all, and that is what the
    // caller has to hear, whatever the two instants are.
    let t9 = Timestamp::from_nanos(9_000_000_000);
    match registry.get_transform_at("gripper", t9, "camera", t9, "map") {
        Err(RegistryError::UnknownFrame(frame)) => assert_eq!(frame, "gripper"),
        other => panic!("expected UnknownFrame, got {other:?}"),
    }
    // With both frames known, the failing leg's frame, instant and covered
    // range are reported under the caller's own target and source names.
    match registry.get_transform_at("camera", t9, "object", t1, "map") {
        Err(RegistryError::NotFoundAt {
            target_frame,
            source_frame,
            frame,
            requested,
            covered,
            ..
        }) => {
            assert_eq!((&*target_frame, &*source_frame), ("camera", "object"));
            assert_eq!(frame, "camera");
            assert_eq!(requested, t9);
            assert_eq!(covered, Some((t1, t2)));
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }

    println!("cross_time: ok");
}
