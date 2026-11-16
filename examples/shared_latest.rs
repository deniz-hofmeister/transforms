//! Share a registry between a writer thread and a reader thread behind an
//! `RwLock`, and look up the newest instant the chain can serve.
//!
//! The writer takes one write guard per update. The reader asks
//! `latest_common_time` and looks that instant up under one read guard, so
//! the writer cannot advance or evict coverage in between. Asking for "now"
//! instead requests an instant newer than the stored data and fails.
//!
//! The lock and the threads belong to this program, not to the crate.
//!
//! Frame tree (parent -> child): odom -> base -> lidar.

use approx::assert_abs_diff_eq;
use core::time::Duration;
use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

const SAMPLES: u32 = 20;
const PERIOD_NANOS: u64 = 100_000_000;

/// The stamp of sample `i`: a fixed 10 Hz stream, not the wall clock.
fn stamp(i: u32) -> Timestamp {
    Timestamp::from_nanos(u64::from(i) * PERIOD_NANOS)
}

fn main() {
    let registry = Arc::new(RwLock::new(Registry::with_max_age(Duration::from_secs(1))));

    // A fixed sensor mount, registered once at startup.
    let base_lidar = Transform::static_between(
        "base",
        "lidar",
        Vector3::new(0.2, 0.0, 0.1),
        Quaternion::identity(),
    )
    .unwrap();
    registry.write().unwrap().add_transform(base_lidar).unwrap();

    // The writer publishes base expressed in odom (parent odom, child
    // base), one sample per write guard, and raises `done` after the last.
    let done = Arc::new(AtomicBool::new(false));
    let writer = {
        let registry = Arc::clone(&registry);
        let done = Arc::clone(&done);
        thread::spawn(move || {
            for i in 1..=SAMPLES {
                let odom_base = Transform::new(
                    "odom",
                    "base",
                    Vector3::new(f64::from(i), 0.0, 0.0),
                    Quaternion::identity(),
                    Stamp::At(stamp(i)),
                )
                .unwrap();
                registry.write().unwrap().add_transform(odom_base).unwrap();
                thread::yield_now();
            }
            done.store(true, Ordering::SeqCst);
        })
    };

    // The reader runs until it has read once after the writer finished. How
    // many samples it sees on the way depends on scheduling; what it asserts
    // about each one, and about the last, does not.
    let reader = thread::spawn(move || {
        let mut newest_seen = None;
        loop {
            // Loaded before the read: if the writer was done by now, this
            // read sees its last sample.
            let writer_done = done.load(Ordering::SeqCst);

            // Both calls under one read guard.
            let guard = registry.read().unwrap();
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
                    // A sample's stamp with that sample's pose, and never
                    // an older one than the previous read.
                    let Stamp::At(t) = odom_lidar.timestamp() else {
                        panic!("a lookup answers at the requested instant");
                    };
                    assert!(newest_seen <= Some(t));
                    let i = u32::try_from(t.as_nanos() / PERIOD_NANOS).unwrap();
                    assert_eq!(t, stamp(i));
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
            if writer_done {
                break;
            }
            thread::yield_now();
        }
        // The read that followed the writer's last insert answered at the
        // newest sample, exactly.
        assert_eq!(newest_seen, Some(stamp(SAMPLES)));
    });

    // Joining both threads carries a panic in either to the exit code.
    writer.join().unwrap();
    reader.join().unwrap();
    println!("reader followed the writer to {:?}", stamp(SAMPLES));
}
