//! Probes reproducing transforms_io 0.8.0 patterns against transforms at
//! this checkout (master f9770d2, 2.3.0).
//!
//! Mirrored source (transforms_io 0.8.0):
//! - ros_time_to_timestamp: src/mt_pubsub_backend.rs:423-428 (identical in
//!   r2r_backend.rs:346-349, ros2_client_backend.rs:289-294, hiroz_backend.rs:280-283)
//! - ros_to_transform:      src/mt_pubsub_backend.rs:370-391
//! - add_transforms:        src/mt_pubsub_backend.rs:315-367 (conflict dedupe);
//!                          r2r_backend.rs:258-290 logs every error at error level
//! - lookup_transform:      src/mt_pubsub_backend.rs:285-307 (target, source);
//!                          r2r_backend.rs:97-119 names them (source, target)
//! - record_statics/static_transforms: src/mt_pubsub_backend.rs:133-147, 272-279

use std::collections::{BTreeMap, HashSet};
use std::time::Duration;
use transforms::Registry;
use transforms::errors::RegistryError;
use transforms::geometry::{Quaternion, Transform, Vector3};
use transforms::time::{Stamp, Timestamp};

#[derive(Clone, Copy, Debug)]
struct Time {
    sec: i32,
    nanosec: u32,
}

#[derive(Clone, Debug)]
struct Tfs {
    stamp: Time,
    frame_id: String,
    child_frame_id: String,
    t: [f64; 3],
    q: [f64; 4], // x y z w, as in geometry_msgs
}

fn tfs(sec: i32, nanosec: u32, parent: &str, child: &str, x: f64) -> Tfs {
    Tfs {
        stamp: Time { sec, nanosec },
        frame_id: parent.into(),
        child_frame_id: child.into(),
        t: [x, 0.0, 0.0],
        q: [0.0, 0.0, 0.0, 1.0],
    }
}

// mirrors mt_pubsub_backend.rs:423-428
fn ros_time_to_timestamp(time: &Time) -> Timestamp {
    let nanos = (time.sec.max(0) as u64) * 1_000_000_000 + (time.nanosec as u64);
    Timestamp::from_nanos(nanos)
}

// mirrors mt_pubsub_backend.rs:370-391
fn ros_to_transform(tf: &Tfs, stamp: Stamp) -> Result<Transform, String> {
    Transform::new(
        &tf.frame_id,
        &tf.child_frame_id,
        Vector3::new(tf.t[0], tf.t[1], tf.t[2]),
        Quaternion { w: tf.q[3], x: tf.q[0], y: tf.q[1], z: tf.q[2] },
        stamp,
    )
    .map_err(|e| format!("InvalidData({e})"))
}

// mirrors mt_pubsub_backend.rs:315-367; returns the log lines instead of logging
fn add_transforms(
    reg: &mut Registry<Timestamp>,
    transforms: Vec<Tfs>,
    stamp_static: bool,
    conflicts: &mut HashSet<(String, String)>,
) -> Vec<String> {
    let mut log = Vec::new();
    for tf in transforms {
        let stamp = if stamp_static {
            Stamp::Static
        } else {
            Stamp::At(ros_time_to_timestamp(&tf.stamp))
        };
        match ros_to_transform(&tf, stamp) {
            Ok(transform) => match reg.add_transform(transform) {
                Ok(()) => {}
                Err(e @ RegistryError::ReparentingNotSupported { .. }) => {
                    if conflicts.insert((tf.frame_id.clone(), tf.child_frame_id.clone())) {
                        log.push(format!(
                            "WARN Ignoring transform {} -> {}: {e}. The first publisher of that frame wins.",
                            tf.frame_id, tf.child_frame_id
                        ));
                    } else {
                        log.push(format!("TRACE Ignoring transform {} -> {}: {e}", tf.frame_id, tf.child_frame_id));
                    }
                }
                Err(e) => log.push(format!(
                    "ERROR Failed to add transform {} -> {}: {e}",
                    tf.frame_id, tf.child_frame_id
                )),
            },
            Err(e) => log.push(format!(
                "ERROR Skipping invalid transform {} -> {}: {e}",
                tf.frame_id, tf.child_frame_id
            )),
        }
    }
    log
}

// mirrors mt_pubsub_backend.rs:285-307 (returns x translation for brevity)
fn lookup_transform(reg: &Registry<Timestamp>, target: &str, source: &str, time: Time) -> Result<(f64, String), String> {
    let timestamp = ros_time_to_timestamp(&time);
    let ts = if time.sec == 0 && time.nanosec == 0 { Timestamp::now() } else { timestamp };
    reg.get_transform(target, source, ts)
        .map(|t| (t.translation().x, format!("{}<-{}", t.parent(), t.child())))
        .map_err(|e| format!("TfError::Lookup({e})"))
}

// mirrors r2r_backend.rs:97-119: parameter names (source_frame, target_frame)
fn r2r_lookup_transform(reg: &Registry<Timestamp>, source_frame: &str, target_frame: &str, time: Time) -> Result<(f64, String), String> {
    let timestamp = ros_time_to_timestamp(&time);
    let ts = if time.sec == 0 && time.nanosec == 0 { Timestamp::now() } else { timestamp };
    reg.get_transform(source_frame, target_frame, ts)
        .map(|t| (t.translation().x, format!("{}<-{}", t.parent(), t.child())))
        .map_err(|e| format!("TfError::Lookup({e})"))
}

fn t(sec: i32, nanosec: u32) -> Time {
    Time { sec, nanosec }
}

fn main() {
    let ten = Duration::from_secs(10);

    println!("== P1 time zero -> Timestamp::now() (mt_pubsub_backend.rs:295-300)");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        add_transforms(&mut reg, vec![tfs(0, 0, "map", "odom", 1.0)], true, &mut c);
        // real-time publisher: newest samples 50 ms and 10 ms ago
        let now = Timestamp::now().as_nanos();
        let s1 = now - 50_000_000;
        let s2 = now - 10_000_000;
        let mk = |n: u64, x| tfs((n / 1_000_000_000) as i32, (n % 1_000_000_000) as u32, "odom", "base", x);
        add_transforms(&mut reg, vec![mk(s1, 1.0), mk(s2, 2.0)], false, &mut c);
        println!("real time, zero-time lookup map<-base: {:?}", lookup_transform(&reg, "map", "base", t(0, 0)));
        let lct = reg.latest_common_time("map", "base");
        println!("latest_common_time map,base: {lct:?}");
        if let Ok(Stamp::At(at)) = lct {
            println!("get_transform at latest_common_time: x = {:?}", reg.get_transform("map", "base", at).map(|t| t.translation().x));
        }
        // sim time: stamps around 100 s
        let mut reg = Registry::with_max_age(ten);
        add_transforms(&mut reg, vec![tfs(100, 0, "odom", "base", 1.0), tfs(101, 0, "odom", "base", 2.0)], false, &mut c);
        println!("sim time, zero-time lookup odom<-base: {:?}", lookup_transform(&reg, "odom", "base", t(0, 0)));
        // static-only chain: zero time works
        let mut reg = Registry::with_max_age(ten);
        add_transforms(&mut reg, vec![tfs(0, 0, "base", "laser", 0.3)], true, &mut c);
        println!("static chain, zero-time lookup base<-laser: {:?}", lookup_transform(&reg, "base", "laser", t(0, 0)));
    }

    println!("\n== P2 static mount re-published under a new parent (static-stale trap, parent flavour)");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        let a = add_transforms(&mut reg, vec![tfs(0, 0, "base_link", "laser", 0.30)], true, &mut c);
        let b = add_transforms(&mut reg, vec![tfs(0, 0, "mast", "laser", 0.05)], true, &mut c);
        let b2 = add_transforms(&mut reg, vec![tfs(0, 0, "mast", "laser", 0.05)], true, &mut c);
        println!("logs: {a:?} {b:?} {b2:?}");
        println!("lookup base_link<-laser at t=50s: {:?}", lookup_transform(&reg, "base_link", "laser", t(50, 0)));
        println!("lookup mast<-laser at t=50s: {:?}", lookup_transform(&reg, "mast", "laser", t(50, 0)));
    }

    println!("\n== P2b same parent, new static pose: retroactive replacement");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        add_transforms(&mut reg, vec![tfs(0, 0, "base_link", "laser", 0.30)], true, &mut c);
        let before = lookup_transform(&reg, "base_link", "laser", t(10, 0));
        add_transforms(&mut reg, vec![tfs(0, 0, "base_link", "laser", 0.35)], true, &mut c);
        println!("at t=10s before re-publish: {before:?}; after re-publish, same t=10s: {:?}", lookup_transform(&reg, "base_link", "laser", t(10, 0)));
    }

    println!("\n== P3 /tf_static first, then the same edge animated on /tf (static-stale trap, kind flavour)");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c_static = HashSet::new();
        let mut c_dyn = HashSet::new();
        add_transforms(&mut reg, vec![tfs(0, 0, "map", "odom", 0.0)], true, &mut c_static);
        let mut logs = Vec::new();
        for i in 0..5 {
            logs.extend(add_transforms(&mut reg, vec![tfs(100 + i, 0, "map", "odom", 1.0 + i as f64)], false, &mut c_dyn));
        }
        println!("{} error lines, first: {:?}", logs.len(), logs.first());
        println!("lookup map<-odom at t=102s (publisher said x=3.0): {:?}", lookup_transform(&reg, "map", "odom", t(102, 0)));
        // reverse order
        let mut reg = Registry::with_max_age(ten);
        add_transforms(&mut reg, vec![tfs(100, 0, "map", "odom", 1.0), tfs(101, 0, "map", "odom", 2.0)], false, &mut c_dyn);
        let l = add_transforms(&mut reg, vec![tfs(0, 0, "map", "odom", 0.0)], true, &mut c_static);
        println!("reverse order (dynamic first) static insert: {l:?}; lookup at 200 s: {:?}", lookup_transform(&reg, "map", "odom", t(200, 0)));
    }

    println!("\n== P4 empty frame_id (ROS 2 default for an unset string)");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        let l = add_transforms(&mut reg, vec![tfs(0, 0, "", "camera", 1.0), tfs(0, 0, "", "lidar", 3.0)], true, &mut c);
        println!("insert logs: {l:?}");
        println!("lookup camera<-lidar: {:?}", lookup_transform(&reg, "camera", "lidar", t(5, 0)));
    }

    println!("\n== P5 negative ROS seconds clamp (sec.max(0))");
    {
        let a = ros_time_to_timestamp(&t(-1, 500_000_000));
        let b = ros_time_to_timestamp(&t(0, 500_000_000));
        let c0 = ros_time_to_timestamp(&t(-7, 500_000_000));
        println!("(-1,0.5e9) -> {} ns, (0,0.5e9) -> {} ns, (-7,0.5e9) -> {} ns", a.as_nanos(), b.as_nanos(), c0.as_nanos());
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        add_transforms(&mut reg, vec![tfs(-1, 500_000_000, "odom", "base", 1.0)], false, &mut c);
        add_transforms(&mut reg, vec![tfs(0, 500_000_000, "odom", "base", 2.0)], false, &mut c);
        println!("lookup at (-1, 0.5e9) after both inserts: {:?}", lookup_transform(&reg, "odom", "base", t(-1, 500_000_000)));
        // nanosec >= 1e9 is accepted and carried into seconds
        println!("(0, 1.5e9) -> {} ns; (1, 0.5e9) -> {} ns", ros_time_to_timestamp(&t(0, 1_500_000_000)).as_nanos(), ros_time_to_timestamp(&t(1, 500_000_000)).as_nanos());
    }

    println!("\n== P6 simulation reset (clock jumps back) with the 10 s default max_age");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        for i in 0..=300u32 {
            let (s, ns) = ((i / 10) as i32, (i % 10) * 100_000_000);
            add_transforms(&mut reg, vec![tfs(s, ns, "odom", "base", f64::from(i) / 10.0)], false, &mut c);
        }
        let mut logs = Vec::new();
        for i in 1..=50u32 {
            let (s, ns) = ((i / 10) as i32, (i % 10) * 100_000_000);
            logs.extend(add_transforms(&mut reg, vec![tfs(s, ns, "odom", "base", 100.0 + f64::from(i) / 10.0)], false, &mut c));
        }
        println!("after reset: {} log lines (all inserts returned Ok)", logs.len());
        println!("lookup at 2.5 s (post-reset data): {:?}", lookup_transform(&reg, "odom", "base", t(2, 500_000_000)));
        println!("latest_common_time: {:?}", reg.latest_common_time("odom", "base"));

        // a jump smaller than max_age interleaves the two epochs
        let mut reg = Registry::with_max_age(ten);
        for i in 0..=80u32 {
            let (s, ns) = ((i / 10) as i32, (i % 10) * 100_000_000);
            add_transforms(&mut reg, vec![tfs(s, ns, "odom", "base", f64::from(i) / 10.0)], false, &mut c);
        }
        for i in 0..=30u32 {
            // the restarted stream publishes at a 50 ms phase offset
            let n = u64::from(i) * 100_000_000 + 50_000_000;
            add_transforms(&mut reg, vec![tfs((n / 1_000_000_000) as i32, (n % 1_000_000_000) as u32, "odom", "base", -10.0)], false, &mut c);
        }
        println!("small jump: lookup at 2.52 s (old epoch says 2.52, new epoch says -10): {:?}", lookup_transform(&reg, "odom", "base", t(2, 520_000_000)));
    }

    println!("\n== P7 quaternion rounding (tf2 accepts |n^2-1| < 1e-2, buffer_core.cpp:66,251-262)");
    {
        for (label, w, z) in [
            ("0.7071/0.7071 (4 decimals)", 0.7071, 0.7071),
            ("0.707/0.707 (3 decimals)", 0.707, 0.707),
            ("0.70710678/0.70710678", 0.70710678, 0.70710678),
            ("f32-rounded exact", f64::from(std::f32::consts::FRAC_1_SQRT_2), f64::from(std::f32::consts::FRAC_1_SQRT_2)),
            ("0.0/0.0 (all-zero message default)", 0.0, 0.0),
        ] {
            let n2: f64 = w * w + z * z;
            let mut m = tfs(0, 0, "base_link", "laser", 0.3);
            m.q = [0.0, 0.0, z, w];
            let r = ros_to_transform(&m, Stamp::Static).map(|_| ());
            println!("{label}: norm-1 = {:.3e}, tf2 accepts = {}, transforms: {:?}", n2.sqrt() - 1.0, (n2 - 1.0).abs() < 10e-3, r);
        }
    }

    println!("\n== P8 r2r/ros2_client/hiroz lookup parameter names (source_frame, target_frame)");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        add_transforms(&mut reg, vec![tfs(0, 0, "base_link", "laser", 0.3)], true, &mut c);
        // a caller trusting the names asks for laser data in base_link
        println!("r2r_lookup(source_frame=\"laser\", target_frame=\"base_link\"): {:?}", r2r_lookup_transform(&reg, "laser", "base_link", t(5, 0)));
        println!("mt_pubsub lookup(target=\"base_link\", source=\"laser\"):           {:?}", lookup_transform(&reg, "base_link", "laser", t(5, 0)));
    }

    println!("\n== P9 publisher dropout inside max_age is interpolated");
    {
        let mut reg = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        add_transforms(&mut reg, vec![tfs(100, 0, "odom", "base", 0.0), tfs(108, 0, "odom", "base", 8.0)], false, &mut c);
        println!("8 s gap, lookup at 104 s: {:?}", lookup_transform(&reg, "odom", "base", t(104, 0)));
    }

    println!("\n== P10 static log (BTreeMap keyed by (parent, child)) replayed vs live registry");
    {
        let mut live = Registry::with_max_age(ten);
        let mut c = HashSet::new();
        let mut statics: BTreeMap<(String, String), Tfs> = BTreeMap::new();
        for m in [tfs(0, 0, "z_mount", "laser", 1.0), tfs(0, 0, "a_mount", "laser", 2.0), tfs(0, 0, "base", "z_mount", 0.0), tfs(0, 0, "base", "a_mount", 0.0)] {
            statics.insert((m.frame_id.clone(), m.child_frame_id.clone()), m.clone());
            add_transforms(&mut live, vec![m], true, &mut c);
        }
        let mut replay = Registry::with_max_age(ten);
        let mut c2 = HashSet::new();
        let logs = add_transforms(&mut replay, statics.values().cloned().collect(), true, &mut c2);
        println!("static_transforms() returns {} entries for 3 children; replay logs: {logs:?}", statics.len());
        println!("live   base<-laser: {:?}", lookup_transform(&live, "base", "laser", t(1, 0)));
        println!("replay base<-laser: {:?}", lookup_transform(&replay, "base", "laser", t(1, 0)));
    }
}
