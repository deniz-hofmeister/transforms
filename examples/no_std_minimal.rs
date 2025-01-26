/// An example on how to add and retrieve transforms
#[cfg(not(feature = "std"))]
fn main() {
    use core::time::Duration;
    use log::{error, info};
    use transforms::{
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
        Registry,
    };

    // Dummy transform generator
    fn generate_transform(t: Timestamp) -> Transform {
        let x = t.as_seconds_unchecked().sin();
        let y = t.as_seconds_unchecked().cos();
        let z = 0.;

        Transform {
            translation: Vector3 { x, y, z },
            rotation: Quaternion {
                w: 1.,
                x: 0.,
                y: 0.,
                z: 0.,
            },
            parent: "a".into(),
            child: "b".into(),
            timestamp: t,
        }
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    let mut registry = Registry::new();

    // Create a transform
    let time = Timestamp::zero();
    let transform = generate_transform(time);

    // Add the transform
    registry.add_transform(transform.clone());

    // Request a transform that is in the future and therefore doesn't exist
    let time_future = (time + Duration::from_secs(1)).unwrap();
    let result = registry.get_transform("a", "b", time_future);
    match result {
        Ok(tf) => info!("Found transform: {:?}", tf),
        Err(e) => error!("Transform not found: {:?}", e),
    }

    // Request the transform that exists
    let result = registry.get_transform("a", "b", time);
    match result {
        Ok(tf) => info!("Found transform: {:?}", tf),
        Err(e) => error!("Transform not found: {:?}", e),
    }

    // Delete all transforms before a certain time
    registry.delete_transforms_before(time);
}

#[cfg(feature = "std")]
fn main() {
    panic!("The 'std' feature must be disabled for this example.");
}
