//! This example demonstrates the complete functionality of the transforms library,
//! including creating transforms, using the registry, and applying transforms to data.
//!
//! This example also showcases the ability of the registry to interpolate transforms for
//! timestamps between known timestamps.
//!
//! This example uses `Registry::new`, which does not automatically remove old
//! transforms; cleanup is done manually with `remove_transforms_before`.

#[cfg(not(feature = "std"))]
fn main() {
    use core::time::Duration;
    use log::{error, info};
    use transforms::{
        Registry, Transform, Transformable,
        geometry::{Point, Quaternion, Vector3},
        time::{Stamp, Timestamp},
    };

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    // Create a transform registry
    let mut registry = Registry::new();
    let time = (Timestamp::zero() + Duration::from_secs(2)).unwrap();

    // Create a point in the camera frame, 2 seconds in
    let mut point = Point::new(
        Vector3::new(0.0, 0.0, 1.0),
        Quaternion::identity(),
        time,
        "camera",
    );
    info!("Created point in camera frame: {point:?}");

    // Create transform from camera to base frame, 1 second ago
    let camera_to_base_t0 = Transform::new(
        "base",
        "camera",
        Vector3::new(0.0, 1.0, 0.0),
        Quaternion::identity(),
        // 1 second before
        Stamp::At((time - Duration::from_secs(1)).unwrap()),
    )
    .unwrap();

    // Create a transform 1 second in the future.
    // This forces the registry to interpolate the values to find
    // the transform for the timestamp of the point object.
    let camera_to_base_t1 = Transform::new(
        "base",
        "camera",
        Vector3::new(0.0, 3.0, 0.0),
        Quaternion::identity(),
        // 1 second in the future
        Stamp::At((time + Duration::from_secs(1)).unwrap()),
    )
    .unwrap();

    // Create transform from base to map frame
    let base_to_map = Transform::new(
        "map",
        "base",
        Vector3::new(2.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(time),
    )
    .unwrap();

    // A fixed sensor mount: static transforms carry `Stamp::Static`, are
    // valid for any query time, and never expire.
    let base_to_imu = Transform::static_between(
        "base",
        "imu",
        Vector3::new(0.1, 0.0, 0.2),
        Quaternion::identity(),
    )
    .unwrap();

    // Add transforms to registry
    registry.add_transform(camera_to_base_t0).unwrap();
    registry.add_transform(camera_to_base_t1).unwrap();
    registry.add_transform(base_to_map).unwrap();
    registry.add_transform(base_to_imu).unwrap();
    info!("Added transforms to registry");

    // The static edge chains with the dynamic ones and serves any time.
    match registry.get_transform("map", "imu", time) {
        Ok(tf) => info!("Static mount resolved in map frame: {tf:?}"),
        Err(e) => error!("Failed to resolve static mount: {e:?}"),
    }

    // Lookup transform for the point, then apply it
    match registry.get_transform_for(&point, "map") {
        Ok(transform) => {
            info!("Retrieved transform from point frame to map: {transform:?}");

            match point.transform(&transform) {
                Ok(()) => info!("Successfully transformed point to map frame: {point:?}"),
                Err(e) => error!("Failed to apply transform to point: {e:?}"),
            }
        }
        Err(e) => error!("Failed to resolve transform for point: {e:?}"),
    }

    // Registry::new() does not automatically wipe old transforms
    // (Registry::with_max_age would)
    // Flush old transforms from the registry
    registry.remove_transforms_before(time);
}

#[cfg(feature = "std")]
fn main() {
    panic!("The 'std' feature must be disabled for this example.");
}
