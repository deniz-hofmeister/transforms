//! This example demonstrates the use of the registry in an async context
//! with concurrent readers and a single writer using an `RwLock`.
//! Readers share access; both readers and writers may wait for the lock.
//!
//! Samples are stamped at publication. Readers query `latest_common_time`
//! and then look up that instant under the same read guard, avoiding a request
//! newer than the stored data.

#[tokio::main]
#[cfg(feature = "std")]
async fn main() {
    use core::time::Duration;
    use log::{error, info};
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use transforms::{
        Registry,
        geometry::{Quaternion, Transform, Vector3},
        time::{Stamp, Timestamp},
    };

    fn generate_transform(t: Timestamp) -> Transform {
        let x = t.as_seconds_lossy().sin();
        let y = t.as_seconds_lossy().cos();
        let z = 0.0;

        Transform::new(
            "a",
            "b",
            Vector3::new(x, y, z),
            Quaternion::identity(),
            Stamp::At(t),
        )
        .unwrap()
    }

    /// How often the writer publishes a new sample. The reader polls at the
    /// same period for the demo's sake, but nothing couples the two: the
    /// reader asks the registry what is servable instead of assuming a lag.
    const PUBLISH_PERIOD: Duration = Duration::from_millis(500);

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("DEBUG")).init();

    let registry = Arc::new(RwLock::new(Registry::with_max_age(Duration::from_secs(10))));

    // A fixed sensor mount: static transforms carry `Stamp::Static`, are
    // valid for any query time, and never expire — registered once at
    // startup, they chain with the dynamic transforms below.
    registry
        .write()
        .await
        .add_transform(
            Transform::static_between(
                "b",
                "lidar",
                Vector3::new(0.2, 0.0, 0.1),
                Quaternion::identity(),
            )
            .unwrap(),
        )
        .unwrap();

    // Writer task - generates and adds transforms (requires exclusive
    // access). Each sample is stamped with the instant it describes.
    let registry_writer = registry.clone();
    let writer = tokio::spawn(async move {
        loop {
            let t = generate_transform(Timestamp::now());
            registry_writer.write().await.add_transform(t).unwrap();
            tokio::time::sleep(PUBLISH_PERIOD).await;
        }
    });

    // Read across the dynamic a -> b edge and static b -> lidar mount.
    // A reader that runs before the first dynamic insert gets UnknownFrame.
    let registry_reader = registry.clone();
    let reader = tokio::spawn(async move {
        loop {
            tokio::time::sleep(PUBLISH_PERIOD).await;

            // Both calls under the same read guard: no writer can advance
            // or evict coverage between "which instant?" and the lookup.
            let guard = registry_reader.read().await;
            let result = guard
                .latest_common_time("a", "lidar")
                .and_then(|stamp| match stamp {
                    Stamp::At(t) => guard.get_transform("a", "lidar", t),
                    // An all-static chain serves any instant the caller
                    // picks; this chain has a dynamic hop, so this arm is
                    // only a matter of completeness.
                    Stamp::Static => guard.get_transform("a", "lidar", Timestamp::now()),
                });
            drop(guard);

            match result {
                Ok(tf) => info!("Found transform: {tf:?}"),
                Err(e) => error!("Transform not found: {e:?}"),
            }
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
