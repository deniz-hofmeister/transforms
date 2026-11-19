//! Express a conveyor object's past position in a later camera frame, then
//! measure the conveyor's displacement between two stored instants.
//! `lookup_across` changes the coordinate reference; it does not predict
//! motion.
//!
//! Frame tree (parent -> child): map -> conveyor -> object, map -> camera.

use approx::assert_abs_diff_eq;
use transforms::{CrossTimeTransform, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

fn main() {
    let t1 = Timestamp::from_nanos(1_000_000_000);
    let t2 = Timestamp::from_nanos(2_000_000_000);
    let mut registry = Registry::unbounded();

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
        let transform =
            Transform::new(parent, child, position, Rotation::IDENTITY, Stamp::At(time)).unwrap();
        registry.insert(&transform).unwrap();
    }

    // Same-time lookups: the object in map at each instant.
    let at_t1 = registry.lookup("map", "object", t1).unwrap();
    let at_t2 = registry.lookup("map", "object", t2).unwrap();
    assert_eq!(at_t1.translation(), Vector3::new(1.0, 0.5, 0.0));
    assert_eq!(at_t2.translation(), Vector3::new(3.0, 0.5, 0.0));

    // "Where was the object at t1, expressed in the camera frame at t2?"
    // `map` is the fixed frame both instants are related through; the
    // caller asserts that it did not move between them. The camera has not
    // moved, so: (1, 0.5, 0) - (0, 2, 0) = (1, -1.5, 0).
    let detection: CrossTimeTransform = registry
        .lookup_across("camera", t2, "object", t1, "map")
        .unwrap();

    // The result is its own type with six accessors: both frames, both
    // instants, and the geometry. It carries no single stamp, so it cannot
    // be inserted into a registry or composed with a same-time `Transform`
    // by accident. The fixed frame is not recorded: it is an assumption of
    // the query.
    assert_eq!(detection.target(), "camera");
    assert_eq!(detection.target_time(), t2);
    assert_eq!(detection.source(), "object");
    assert_eq!(detection.source_time(), t1);
    assert_eq!(detection.translation(), Vector3::new(1.0, -1.5, 0.0));
    assert_eq!(detection.rotation(), Rotation::IDENTITY);

    // Apply it with the recipe from the `Transform` documentation: a point
    // 10 cm along the object's x axis at t1, in camera coordinates at t2.
    let p = Vector3::new(0.1, 0.0, 0.0);
    let r = detection.rotation().rotate(p);
    let t = detection.translation();
    let p_camera = Vector3::new(r.x + t.x, r.y + t.y, r.z + t.z);
    assert_abs_diff_eq!(p_camera.x, 1.1, epsilon = 1e-12);
    assert_abs_diff_eq!(p_camera.y, -1.5, epsilon = 1e-12);
    assert_abs_diff_eq!(p_camera.z, 0.0, epsilon = 1e-12);

    // The contrast: a same-time lookup at t2 says where the object is at
    // t2, after the conveyor carried it 2 m further. Using it for a
    // detection made at t1 would be off by that distance.
    let same_time = registry.lookup("camera", "object", t2).unwrap();
    assert_eq!(same_time.stamp(), Stamp::At(t2));
    assert_eq!(same_time.translation(), Vector3::new(3.0, -1.5, 0.0));
    assert_ne!(same_time.translation(), detection.translation());

    // "Where is the object at t2, expressed in its own frame at t1?"
    // (3, 0.5, 0) - (1, 0.5, 0) = (2, 0, 0): the conveyor's displacement.
    // A same-time lookup of a frame relative to itself is the identity.
    let displacement = registry
        .lookup_across("object", t1, "object", t2, "map")
        .unwrap();
    assert_eq!(displacement.translation(), Vector3::new(2.0, 0.0, 0.0));
    let itself = registry.lookup("object", "object", t2).unwrap();
    assert_eq!(itself.translation(), Vector3::new(0.0, 0.0, 0.0));

    println!(
        "object at t1 in camera at t2: {:?}",
        detection.translation()
    );
    println!(
        "object at t2 in camera at t2: {:?}",
        same_time.translation()
    );
    println!("conveyor displacement: {:?}", displacement.translation());
}
