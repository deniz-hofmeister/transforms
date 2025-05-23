//! This example demonstrates the complete functionality of the transforms library,
//! including creating transforms, using the registry, and applying transforms to data.
//!
//! This example also showcases the ability of the registry to interpolate transforms for
//! timestamps between known timestamps.

#[cfg(feature = "std")]
fn main() {
    use core::time::Duration;
    use log::{error, info};
    use transforms::{
        geometry::{Point, Quaternion, Vector3},
        time::Timestamp,
        Registry, Transform, Transformable,
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    // Create a transform registry with 10 second max_age
    let mut registry = Registry::new(Duration::from_secs(10));

    // Timestamp::now() is not available in no_std
    let time = Timestamp::now();

    // Create a point in the camera frame
    let mut point = Point {
        position: Vector3::new(0.0, 0.0, 1.0),
        orientation: Quaternion::identity(),
        timestamp: time,
        frame: "camera".into(),
    };
    info!("Created point in camera frame: {:?}", point);

    // Create transform from camera to base frame, 1 second ago
    let camera_to_base_t0 = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        // 1 second ago
        timestamp: (time - Duration::from_secs(1)).unwrap(),
        parent: "base".into(),
        child: "camera".into(),
    };

    // Create a transform 1 second in the future.
    // This forces the registry to interpolate the values to find
    // the transform for the timestamp of the point object.
    let camera_to_base_t1 = Transform {
        translation: Vector3::new(0.0, 3.0, 0.0),
        rotation: Quaternion::identity(),
        // 1 second in the future
        timestamp: (time + Duration::from_secs(1)).unwrap(),
        parent: "base".into(),
        child: "camera".into(),
    };

    // Create transform from base to map frame
    let base_to_map = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: time,
        parent: "map".into(),
        child: "base".into(),
    };

    // Add transforms to registry
    registry.add_transform(camera_to_base_t0);
    registry.add_transform(camera_to_base_t1);
    registry.add_transform(base_to_map);
    info!("Added transforms to registry");

    // Get transform from camera to map frame
    match registry.get_transform("camera", "map", time) {
        Ok(transform) => {
            info!("Retrieved transform from camera to map: {:?}", transform);

            // Apply transform to point
            match point.transform(&transform.inverse().expect("Failed to invert")) {
                Ok(()) => info!("Successfully transformed point to map frame: {:?}", point),
                Err(e) => error!("Failed to transform point: {:?}", e),
            }
        }
        Err(e) => error!("Failed to get transform: {:?}", e),
    }
}

#[cfg(not(feature = "std"))]
fn main() {
    panic!("The 'std' feature must be enabled for this example.");
}
