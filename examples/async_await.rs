#[cfg(feature = "async")]
mod async_minimal {
    pub use log::{error, info};
    pub use std::sync::Arc;
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
}

#[cfg(feature = "async")]
use async_minimal::*;

#[cfg(feature = "async")]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    // Create a new transform registry with a time-to-live of 10 seconds. Transforms older than
    // 10 seconds will be flushed. Mutex is not needed as mutex is managed internally.
    let ttl = Duration::from_secs(10);
    let registry = Arc::new(Registry::new(ttl.into()));

    // Writer task - generates and adds transforms
    let registry_writer = Arc::clone(&registry);
    let writer = tokio::spawn(async move {
        info!("Writer adding new transform");
        loop {
            // Create a transform
            let time = Timestamp::now();
            let t = generate_transform(time);

            // Add the transform and catch potential errors
            if let Err(e) = registry_writer.add_transform(t.clone()).await {
                error!("Error adding transform: {:?}", e);
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // Reader task - uses await_transform to wait for transforms
    let registry_reader = Arc::clone(&registry);
    let reader = tokio::spawn(async move {
        info!("Reader waiting for a new transform");
        loop {
            // Request a transform in the future, therefore forcing it to wait.
            let time = (Timestamp::now() + Duration::from_secs(1).into()).unwrap();

            // Wait for the transform to become available
            match registry_reader.await_transform("a", "b", time).await {
                Ok(tf) => info!("Found transform through await: {:?}", tf),
                Err(e) => error!("Error waiting for transform: {:?}", e),
            }
        }
    });

    let _ = tokio::join!(writer, reader);
}

#[cfg(not(feature = "async"))]
fn main() {
    panic!(
        "This example requires the 'async' feature. Please run with: cargo run --example minimal_async --features async"
    );
}
