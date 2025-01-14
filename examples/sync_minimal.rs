/// An example on how to add and retrieve transforms
#[cfg(not(feature = "async"))]
fn main() {
    use log::{error, info};
    use std::time::Duration;
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

    // Create a new transform registry with a time-to-live of 10 seconds.
    let max_age = Duration::from_secs(10);

    // We do not need to use Arc or Mutex as this example is fully synchronous.
    let mut registry = Registry::new(max_age);

    // Create a transform
    let time = Timestamp::now();
    let transform = generate_transform(time);

    // Add the transform
    if let Err(e) = registry.add_transform(transform.clone()) {
        error!("Error adding transform: {:?}", e);
    }

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
}

#[cfg(feature = "async")]
fn main() {
    panic!("This example should not be run with the 'async' feature enabled.");
}
