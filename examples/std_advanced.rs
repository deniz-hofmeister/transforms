//! This example demonstrates the "time travel" feature of the transforms library
//! using a conveyor belt scenario.
//!
//! Scenario: A camera mounted on a robot arm detects an object on a moving conveyor belt.
//! The detection happened at t1, but we want to know where that object is now (at t2),
//! given that the conveyor has moved. The "map" frame is stationary and serves as
//! the fixed reference frame.
//!
//! Frame tree: map -> conveyor -> object
//!                 \-> camera

#[cfg(feature = "std")]
fn main() {
    use core::time::Duration;
    use log::info;
    use transforms::{
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
        Registry,
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    let mut registry = Registry::new(Duration::from_secs(10));

    let t1 = Timestamp::now();
    let t2 = (t1 + Duration::from_secs(1)).unwrap();

    // The conveyor belt is at x=1 in the map frame at t1
    registry.add_transform(Transform {
        translation: Vector3 {
            x: 1.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: t1,
        parent: "map".into(),
        child: "conveyor".into(),
    });

    // By t2 the conveyor has moved to x=3
    registry.add_transform(Transform {
        translation: Vector3 {
            x: 3.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: t2,
        parent: "map".into(),
        child: "conveyor".into(),
    });

    // The object sits at y=0.5 on the conveyor (same at both times)
    for &t in &[t1, t2] {
        registry.add_transform(Transform {
            translation: Vector3 {
                x: 0.,
                y: 0.5,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t,
            parent: "conveyor".into(),
            child: "object".into(),
        });
    }

    // The camera is fixed in the map frame at (0, 2, 0)
    for &t in &[t1, t2] {
        registry.add_transform(Transform {
            translation: Vector3 {
                x: 0.,
                y: 2.,
                z: 0.,
            },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            timestamp: t,
            parent: "map".into(),
            child: "camera".into(),
        });
    }

    // --- Regular lookups ---
    // At t1 the object is at (1, 0.5, 0) in map
    let object_in_map_t1 = registry
        .get_transform("map", "object", t1)
        .expect("lookup at t1 failed");
    info!("Object in map at t1: {:?}", object_in_map_t1.translation);

    // At t2 the object is at (3, 0.5, 0) in map
    let object_in_map_t2 = registry
        .get_transform("map", "object", t2)
        .expect("lookup at t2 failed");
    info!("Object in map at t2: {:?}", object_in_map_t2.translation);

    // --- Time travel ---
    // "Where was the object at t1, expressed in the camera frame at t2?"
    // The camera hasn't moved, so the answer is the object's t1 map position
    // relative to the camera: (1, 0.5, 0) - (0, 2, 0) = (1, -1.5, 0)
    let result = registry
        .get_transform_at(
            "camera", // target_frame
            t2,       // target_time
            "object", // source_frame
            t1,       // source_time  (when the detection happened)
            "map",    // fixed_frame
        )
        .expect("time travel lookup failed");
    info!(
        "Object-at-t1 in camera-at-t2 (time travel): {:?}",
        result.translation
    );

    // "Where is the object now (t2), expressed in its own position at t1?"
    // Object at t1 was at (1, 0.5, 0) in map. Object at t2 is at (3, 0.5, 0).
    // So object-at-t2 in object-at-t1's frame = (3, 0.5, 0) - (1, 0.5, 0) = (2, 0, 0)
    let drift = registry
        .get_transform_at(
            "object", // target_frame
            t1,       // target_time  (the old frame)
            "object", // source_frame
            t2,       // source_time  (where it is now)
            "map",    // fixed_frame
        )
        .expect("drift lookup failed");
    info!("Conveyor drift (t1 -> t2): {:?}", drift.translation);
}

#[cfg(not(feature = "std"))]
fn main() {
    panic!("The 'std' feature must be enabled for this example.");
}
