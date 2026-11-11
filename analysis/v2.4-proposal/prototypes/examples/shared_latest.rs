//! Share a registry between a writer and a reader behind an `RwLock`, and
//! look up the newest instant the chain can serve.
//!
//! The reader asks `latest_common_time` and looks that instant up under the
//! same read guard, so no writer can advance or evict coverage in between.
//! Asking for "now" instead requests an instant newer than the stored data
//! and fails.
//!
//! Frame tree (parent -> child): odom -> base -> lidar.

use approx::assert_abs_diff_eq;
use core::time::Duration;
use std::sync::Arc;
use tokio::sync::RwLock;
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

const SAMPLES: u32 = 20;
const PERIOD: Duration = Duration::from_millis(5);

/// The stamp of sample `i`: a fixed 10 Hz stream, not the wall clock.
fn stamp(i: u32) -> Timestamp {
    Timestamp::from_nanos(u64::from(i) * 100_000_000)
}

#[tokio::main]
async fn main() {
    let registry = Arc::new(RwLock::new(Registry::with_max_age(Duration::from_secs(1))));

    // A fixed sensor mount, registered once at startup.
    let base_lidar = Transform::static_between(
        "base",
        "lidar",
        Vector3::new(0.2, 0.0, 0.1),
        Quaternion::identity(),
    )
    .unwrap();
    registry.write().await.add_transform(base_lidar).unwrap();

    // The writer publishes base expressed in odom, one sample per period.
    let writer_registry = registry.clone();
    let writer = tokio::spawn(async move {
        for i in 1..=SAMPLES {
            let odom_base = Transform::new(
                "odom",
                "base",
                Vector3::new(f64::from(i), 0.0, 0.0),
                Quaternion::identity(),
                Stamp::At(stamp(i)),
            )
            .unwrap();
            writer_registry
                .write()
                .await
                .add_transform(odom_base)
                .unwrap();
            tokio::time::sleep(PERIOD).await;
        }
    });

    // The reader runs until it has seen the writer's last sample.
    let reader = tokio::spawn(async move {
        let mut newest_seen = None;
        while newest_seen != Some(stamp(SAMPLES)) {
            // Both calls under one read guard.
            let guard = registry.read().await;
            let result =
                guard
                    .latest_common_time("odom", "lidar")
                    .and_then(|latest| match latest {
                        Stamp::At(t) => guard.get_transform("odom", "lidar", t),
                        // An all-static chain serves every instant, so the
                        // caller passes one of its own. This chain has a
                        // dynamic hop and never takes this arm, but an
                        // `.at().unwrap()` here would panic on one that does.
                        Stamp::Static => guard.get_transform("odom", "lidar", stamp(0)),
                    });
            drop(guard);

            match result {
                Ok(odom_lidar) => {
                    // The newest sample, exactly, and never an older one
                    // than the previous read.
                    let t = odom_lidar.timestamp().at().unwrap();
                    assert!(newest_seen <= Some(t));
                    let i = u32::try_from(t.as_nanos() / 100_000_000).unwrap();
                    assert_abs_diff_eq!(
                        odom_lidar.translation(),
                        Vector3::new(f64::from(i) + 0.2, 0.0, 0.1),
                        epsilon = 1e-12
                    );
                    newest_seen = Some(t);
                }
                // Before the writer's first sample `odom` is not in the
                // tree yet. Nothing else may fail.
                Err(RegistryError::UnknownFrame(frame)) => assert_eq!(frame, "odom"),
                Err(other) => panic!("unexpected lookup error: {other}"),
            }
            tokio::time::sleep(PERIOD).await;
        }
    });

    writer.await.unwrap();
    reader.await.unwrap();
    println!("reader followed the writer to {:?}", stamp(SAMPLES));
}
