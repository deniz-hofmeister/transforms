pub use log::{error, info};
pub use std::time::Duration;
pub use transforms::types::{Quaternion, Registry, Timestamp, Transform, Vector3};

// Dummy transform generator
pub fn generate_transform(t: Timestamp) -> Transform {
    let x = t.as_seconds().unwrap().sin();
    let y = t.as_seconds().unwrap().cos();
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

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    // Create a new transform registry with a time-to-live of 10 seconds. Transforms older than
    // 10 seconds will be flushed.
    let ttl = Duration::from_secs(10);
    let mut registry = Registry::new(ttl.into());

    // Create a transform
    let time = Timestamp::now();
    let transform = generate_transform(time);

    // Add the transform
    if let Err(e) = registry.add_transform(transform.clone()) {
        error!("Error adding transform: {:?}", e);
    }

    // Request a transform that is in the future and therefore doesn't exist
    let time_future = (time + Duration::from_secs(1).into()).unwrap();
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
