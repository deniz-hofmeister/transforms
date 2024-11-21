use log::{error, info};
use std::sync::Arc;
use transforms::types::{Duration, Quaternion, Registry, Timestamp, Transform, Vector3};

/// Dummy transform generator
fn generate_transform(t: Timestamp) -> Transform {
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

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    let ttl = std::time::Duration::from_secs(10);
    let mut registry = Arc::new(Registry::new(ttl.into()));

    // Writer task - generates and adds transforms
    let mut registry_writer = registry.clone();
    let writer = tokio::spawn(async move {
        loop {
            let time = Timestamp::now();
            let t = generate_transform(time);
            if let Err(e) = registry_writer.add_transform(t.clone()).await {
                error!("Error adding transform: {:?}", e);
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    // Reader task - uses await_transform to wait for transforms
    let mut registry_reader = registry.clone();
    let reader = tokio::spawn(async move {
        info!("Running reader");
        loop {
            let time = (Timestamp::now() - Duration::try_from(1.0).unwrap()).unwrap();
            match registry_reader.await_transform("a", "b", time).await {
                Ok(tf) => info!("Found transform through await: {:?}", tf),
                Err(e) => error!("Error waiting for transform: {:?}", e),
            }
        }
    });

    // Run both tasks
    let _ = tokio::join!(writer, reader);
}
