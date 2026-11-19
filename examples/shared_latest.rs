//! Share a registry between a writer thread and a reader thread behind an
//! `RwLock`, and look up the newest instant the chain can serve.
//!
//! The writer takes one write guard per update. The reader asks
//! `latest_common_time` and looks that instant up under one read guard, so
//! the writer cannot advance or remove coverage in between. Asking for
//! "now" instead requests an instant newer than the stored data and fails.
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
use transforms::{LookupError, Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

const SAMPLES: i32 = 20;
const PERIOD_NANOS: i64 = 100_000_000;

/// The stamp of sample `i`: a fixed 10 Hz stream, not the wall clock.
fn stamp(i: i32) -> Timestamp {
    Timestamp::from_nanos(i64::from(i) * PERIOD_NANOS)
}

/// The freshest transform the chain can serve. The caller holds the guard
/// that `registry` borrows from, so both calls see one state of the tree.
fn latest(
    registry: &Registry,
    target: &str,
    source: &str,
) -> Result<Transform, LookupError> {
    let at = match registry.latest_common_time(target, source)? {
        Stamp::At(newest) => newest,
        // Every edge on the chain is static and serves every instant, so
        // any instant is a valid one to ask for. The chain in this example
        // has a dynamic hop and never takes this arm; the reader below
        // still needs it for a chain that does.
        Stamp::Static => Timestamp::from_nanos(0),
    };
    registry.lookup(target, source, at)
}

fn main() {
    let registry = Arc::new(RwLock::new(Registry::with_max_age(Duration::from_secs(1))));

    // A fixed sensor mount, registered once at startup.
    let base_lidar = Transform::new(
        "base",
        "lidar",
        Vector3::new(0.2, 0.0, 0.1),
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap();
    registry.write().unwrap().insert(&base_lidar).unwrap();

    // An all-static chain: `latest_common_time` answers `Static`, and the
    // lookup at the instant the `Static` arm picked succeeds.
    {
        let guard = registry.read().unwrap();
        assert_eq!(guard.latest_common_time("base", "lidar"), Ok(Stamp::Static));
        let mount = latest(&guard, "base", "lidar").unwrap();
        assert_eq!(mount.stamp(), Stamp::At(Timestamp::from_nanos(0)));
        assert_eq!(mount.translation(), base_lidar.translation());
    }

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
                    Rotation::IDENTITY,
                    Stamp::At(stamp(i)),
                )
                .unwrap();
                registry.write().unwrap().insert(&odom_base).unwrap();
                thread::yield_now();
            }
            done.store(true, Ordering::SeqCst);
        })
    };

    // The reader runs until it has read once after the writer finished. How
    // many samples it sees on the way depends on scheduling; what it asserts
    // about each one, and about the last, does not.
    let reader = {
        let registry = Arc::clone(&registry);
        thread::spawn(move || {
            let mut newest_seen = None;
            loop {
                // Loaded before the read: if the writer was done by now,
                // this read sees its last sample.
                let writer_done = done.load(Ordering::SeqCst);

                // Both calls under one read guard. With a guard per call
                // the writer could insert in between, and a bounded
                // registry could have moved its window past the instant
                // the first call returned.
                let guard = registry.read().unwrap();
                let result = latest(&guard, "odom", "lidar");
                drop(guard);

                match result {
                    Ok(odom_lidar) => {
                        // A sample's stamp with that sample's pose, and
                        // never an older one than the previous read.
                        let Stamp::At(t) = odom_lidar.stamp() else {
                            panic!("a lookup answers at the requested instant");
                        };
                        assert!(newest_seen <= Some(t));
                        let i = i32::try_from(t.as_nanos() / PERIOD_NANOS).unwrap();
                        assert_eq!(t, stamp(i));
                        let translation = odom_lidar.translation();
                        assert_abs_diff_eq!(translation.x, f64::from(i) + 0.2, epsilon = 1e-12);
                        assert_abs_diff_eq!(translation.y, 0.0, epsilon = 1e-12);
                        assert_abs_diff_eq!(translation.z, 0.1, epsilon = 1e-12);
                        newest_seen = Some(t);
                    }
                    // Before the writer's first sample no stored edge names
                    // `odom`, so it is not known yet. Nothing else may fail.
                    Err(LookupError::UnknownFrame { frame, .. }) => assert_eq!(frame, "odom"),
                    Err(other) => panic!("unexpected lookup error: {other}"),
                }
                if writer_done {
                    break;
                }
                thread::yield_now();
            }
            // The read that followed the writer's last insert answered at
            // the newest sample, exactly.
            assert_eq!(newest_seen, Some(stamp(SAMPLES)));
        })
    };

    // Joining both threads carries a panic in either to the exit code.
    writer.join().unwrap();
    reader.join().unwrap();

    // Why the wall clock is the wrong instant. A reader's clock runs ahead
    // of the newest stored stamp by the publishing and transport delay;
    // here it stands 30 ms past the last sample. There is no
    // extrapolation, so that lookup fails on every tick, while the instant
    // `latest_common_time` names is served.
    let guard = registry.read().unwrap();
    let now = stamp(SAMPLES)
        .checked_add(Duration::from_millis(30))
        .unwrap();
    match guard.lookup("odom", "lidar", now) {
        Err(LookupError::OutOfRange {
            frame,
            requested,
            latest,
            ..
        }) => {
            assert_eq!(frame, "base");
            assert_eq!((requested, latest), (now, stamp(SAMPLES)));
        }
        other => panic!("expected OutOfRange, got {other:?}"),
    }
    assert_eq!(
        guard.latest_common_time("odom", "lidar"),
        Ok(Stamp::At(stamp(SAMPLES)))
    );
    assert!(latest(&guard, "odom", "lidar").is_ok());

    println!("reader followed the writer to {} s", stamp(SAMPLES));
    println!("a lookup at {now} s, the reader's clock, is refused");
}
