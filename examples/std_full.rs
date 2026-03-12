/// This example demonstrates the use of the registry in an async context
/// with concurrent readers and a single writer, using an `RwLock` to allow
/// multiple readers to query transforms simultaneously without blocking
/// each other.

#[tokio::main]
#[cfg(feature = "std")]
async fn main() {
    use core::time::Duration;
    use log::{error, info};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use transforms::{
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
        Registry,
    };

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

    let registry = Arc::new(RwLock::new(Registry::new(Duration::from_secs(10))));

    // Writer task - generates and adds transforms (requires exclusive access)
    let registry_writer = registry.clone();
    let writer = tokio::spawn(async move {
        loop {
            let time = (Timestamp::now() + Duration::from_secs(1)).unwrap();
            let t = generate_transform(time);
            registry_writer.write().await.add_transform(t);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // Reader task - queries transforms (shared access, does not block other readers)
    let registry_reader = registry.clone();
    let reader = tokio::spawn(async move {
        loop {
            let result = registry_reader
                .read()
                .await
                .get_transform("a", "b", Timestamp::now());
            match result {
                Ok(tf) => info!("Found transform: {:?}", tf),
                Err(e) => error!("Transform not found: {:?}", e),
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // Run example for a fixed amount of time
    tokio::time::sleep(Duration::from_secs(5)).await;
    writer.abort();
    reader.abort();
}

#[cfg(not(feature = "std"))]
fn main() {
    panic!("The 'std' feature must be enabled for this example.");
}
