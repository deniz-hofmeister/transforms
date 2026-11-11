//! Express a conveyor object's past position in a later camera frame, then
//! measure the conveyor's displacement between two stored instants.
//! `get_transform_at` changes the coordinate reference; it does not predict
//! motion.
//!
//! Frame tree (parent -> child): map -> conveyor -> object, map -> camera.

use transforms::{
    Registry,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

fn main() {
    let t1 = Timestamp::from_nanos(1_000_000_000);
    let t2 = Timestamp::from_nanos(2_000_000_000);
    let mut registry = Registry::new();

    // The conveyor moves from x = 1 to x = 3 in map; the object rides it at
    // y = 0.5; the camera stays at (0, 2, 0) in map.
    for (parent, child, time, position) in [
        ("map", "conveyor", t1, Vector3::new(1.0, 0.0, 0.0)),
        ("map", "conveyor", t2, Vector3::new(3.0, 0.0, 0.0)),
        ("conveyor", "object", t1, Vector3::new(0.0, 0.5, 0.0)),
        ("conveyor", "object", t2, Vector3::new(0.0, 0.5, 0.0)),
        ("map", "camera", t1, Vector3::new(0.0, 2.0, 0.0)),
        ("map", "camera", t2, Vector3::new(0.0, 2.0, 0.0)),
    ] {
        let transform = Transform::new(
            parent,
            child,
            position,
            Quaternion::identity(),
            Stamp::At(time),
        )
        .unwrap();
        registry.add_transform(transform).unwrap();
    }

    // Same-time lookups: the object in map at each instant.
    let at_t1 = registry.get_transform("map", "object", t1).unwrap();
    let at_t2 = registry.get_transform("map", "object", t2).unwrap();
    assert_eq!(at_t1.translation(), Vector3::new(1.0, 0.5, 0.0));
    assert_eq!(at_t2.translation(), Vector3::new(3.0, 0.5, 0.0));

    // "Where was the object at t1, expressed in the camera frame at t2?"
    // `map` is the fixed frame both instants are related through. The camera
    // has not moved, so: (1, 0.5, 0) - (0, 2, 0) = (1, -1.5, 0).
    let detection = registry
        .get_transform_at("camera", t2, "object", t1, "map")
        .unwrap();
    assert_eq!(detection.translation(), Vector3::new(1.0, -1.5, 0.0));

    // The result is stamped with the target time only. Keep the source time
    // beside it: re-inserting it as a single-time sample, or applying it to
    // data stamped t2, is a mistake nothing can check.
    assert_eq!(detection.timestamp(), Stamp::At(t2));

    // "Where is the object at t2, expressed in its own frame at t1?"
    // (3, 0.5, 0) - (1, 0.5, 0) = (2, 0, 0): the conveyor's displacement.
    let displacement = registry
        .get_transform_at("object", t1, "object", t2, "map")
        .unwrap();
    assert_eq!(displacement.translation(), Vector3::new(2.0, 0.0, 0.0));

    println!(
        "object at t1 in camera at t2: {:?}",
        detection.translation()
    );
    println!("conveyor displacement: {:?}", displacement.translation());
}
