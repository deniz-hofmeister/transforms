//! One writer, several readers, and a "latest" lookup that cannot race.
//!
//! A reader asks `latest_common_time` which instant the chain can serve and
//! looks that instant up under the same read guard, so no insert can move
//! or evict coverage between the two calls. Looking up at the wall clock
//! instead fails on any chain with a dynamic hop.
//!
//! Runs unchanged with and without the crate's `std` feature; the lock and
//! the threads belong to this program, not to the crate.

use approx::assert_abs_diff_eq;
use core::time::Duration;
use std::{
    sync::{Arc, RwLock},
    thread,
};
use transforms::{
    Registry, Transform,
    errors::RegistryError,
    geometry::{Quaternion, Vector3},
    time::{Stamp, Timestamp},
};

const SAMPLES: u32 = 200;
const PERIOD_NANOS: u64 = 10_000_000;

/// The pose published at sample `k`: x advances 1 m per sample.
fn sample(k: u32) -> Transform {
    Transform::new(
        "odom",
        "base",
        Vector3::new(f64::from(k), 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(Timestamp::from_nanos(u64::from(k) * PERIOD_NANOS)),
    )
    .unwrap()
}

fn main() {
    let registry = Arc::new(RwLock::new(Registry::with_max_age(Duration::from_secs(1))));

    registry
        .write()
        .unwrap()
        .add_transform(
            Transform::static_between(
                "base",
                "lidar",
                Vector3::new(0.2, 0.0, 0.1),
                Quaternion::identity(),
            )
            .unwrap(),
        )
        .unwrap();

    let writer = {
        let registry = Arc::clone(&registry);
        thread::spawn(move || {
            for k in 0..SAMPLES {
                registry.write().unwrap().add_transform(sample(k)).unwrap();
                thread::yield_now();
            }
        })
    };

    let readers: Vec<_> = (0..4)
        .map(|_| {
            let registry = Arc::clone(&registry);
            thread::spawn(move || {
                let mut served = 0_u64;
                loop {
                    let guard = registry.read().unwrap();
                    let result =
                        guard
                            .latest_common_time("odom", "lidar")
                            .and_then(|stamp| match stamp {
                                Stamp::At(t) => guard.get_transform("odom", "lidar", t),
                                // An all-static chain serves any instant; this
                                // chain has a dynamic hop, so it is not reached.
                                Stamp::Static => unreachable!("odom -> base is dynamic"),
                            });
                    drop(guard);

                    match result {
                        Ok(tf) => {
                            // The newest sample, exactly: no interpolation
                            // and no stale answer.
                            let Stamp::At(t) = tf.timestamp() else {
                                panic!("a dynamic chain answers at an instant");
                            };
                            let k = u32::try_from(t.as_nanos() / PERIOD_NANOS).unwrap();
                            assert_abs_diff_eq!(
                                tf.translation(),
                                Vector3::new(f64::from(k) + 0.2, 0.0, 0.1),
                                epsilon = 1e-12
                            );
                            served += 1;
                            if k == SAMPLES - 1 {
                                return served;
                            }
                        }
                        // Before the first dynamic insert `odom` is not a
                        // frame yet. Nothing else is acceptable here.
                        Err(RegistryError::UnknownFrame(frame)) => assert_eq!(frame, "odom"),
                        Err(other) => panic!("unexpected lookup failure: {other}"),
                    }
                    thread::yield_now();
                }
            })
        })
        .collect();

    writer.join().unwrap();
    for reader in readers {
        assert!(reader.join().unwrap() > 0);
    }

    println!("shared_latest: ok");
}
