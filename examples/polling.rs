/// This example demonstrates the use of sync implementation of the registry in an async main
/// to add and retrieve transforms.
fn main() {
    use log::{error, info};
    use std::{sync::Arc, time::Duration};
    use tokio::sync::Mutex;
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

    // Create a new transform registry with a time-to-live of 10 seconds. Transforms older than
    // 10 seconds will be flushed.
    let max_age = Duration::from_secs(10);

    // Arc and Mutex is used in this example because we load the synchronous implementation of the
    // registry, but in a multi-threaded context.
    let registry = Arc::new(Mutex::new(Registry::new(max_age)));

    // Writer task - generates and adds transforms
    let registry_writer = registry.clone();
    let writer = tokio::spawn(async move {
        loop {
            let time = Timestamp::now();
            let t = generate_transform(time);
            let mut r = registry_writer.lock().await;

            // Add the transform to the registry
            if let Err(e) = r.add_transform(t.clone()) {
                error!("Error adding transform: {:?}", e);
            }
            drop(r);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // Reader task - uses get_transform to poll for transforms
    let registry_reader = registry.clone();
    let reader = tokio::spawn(async move {
        loop {
            // Request a transform in the past, which will be unavailable initially.
            let time = (Timestamp::now() - Duration::from_secs(1)).unwrap();
            let mut r = registry_reader.lock().await;

            // Poll the registry for the transform
            let result = r.get_transform("a", "b", time);
            match result {
                Ok(tf) => info!("Found transform: {:?}", tf),
                Err(e) => error!("Transform not found: {:?}", e),
            }
            drop(r);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // Run example for a fixed amount of time
    tokio::time::sleep(Duration::from_secs(5)).await;
    writer.abort();
    reader.abort();
}
