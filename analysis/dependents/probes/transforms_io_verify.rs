//! Independent re-probe of transforms_io 0.8.0 behaviour on transforms master.
//! Mirrors (copied logic, ROS message types replaced by plain structs):
//!   ros_time_to_timestamp  tio@0.8.0 src/mt_pubsub_backend.rs:423-428 (same in r2r:346-349)
//!   ros_to_transform       mt_pubsub_backend.rs:370-391
//!   add_transforms (mt)    mt_pubsub_backend.rs:315-367 (dedupe of ReparentingNotSupported)
//!   add_transforms (r2r)   r2r_backend.rs:258-290 (error per rejection)
//!   lookup_transform (mt)  mt_pubsub_backend.rs:285-307
//!   lookup_transform (r2r) r2r_backend.rs:97-119
//!   record_statics/static_transforms mt_pubsub_backend.rs:133-147,271-279
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

use transforms::Registry;
use transforms::errors::RegistryError;
use transforms::geometry::{Quaternion, Transform, Vector3};
use transforms::time::{Stamp, Timestamp};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Time {
    sec: i32,
    nanosec: u32,
}
#[derive(Clone, Debug)]
struct Tf {
    frame_id: String,
    stamp: Time,
    child: String,
    x: f64,
    q: [f64; 4], // w x y z
}
fn tf(p: &str, c: &str, stamp: Time, x: f64) -> Tf {
    Tf { frame_id: p.into(), stamp, child: c.into(), x, q: [1.0, 0.0, 0.0, 0.0] }
}
fn secs(s: f64) -> Time {
    let n = (s * 1e9).round() as i64;
    Time { sec: (n.div_euclid(1_000_000_000)) as i32, nanosec: n.rem_euclid(1_000_000_000) as u32 }
}
fn ts_from_ns(ns: u64) -> Time {
    Time { sec: (ns / 1_000_000_000) as i32, nanosec: (ns % 1_000_000_000) as u32 }
}

fn ros_time_to_timestamp(time: &Time) -> Timestamp {
    let nanos = (time.sec.max(0) as u64) * 1_000_000_000 + (time.nanosec as u64);
    Timestamp::from_nanos(nanos)
}
fn ros_to_transform(t: &Tf, stamp: Stamp) -> Result<Transform, String> {
    Transform::new(
        &t.frame_id,
        &t.child,
        Vector3::new(t.x, 0.0, 0.0),
        Quaternion { w: t.q[0], x: t.q[1], y: t.q[2], z: t.q[3] },
        stamp,
    )
    .map_err(|e| format!("InvalidData({e})"))
}

fn add_mt(reg: &mut Registry<Timestamp>, msg: Vec<Tf>, stamp_static: bool, conflicts: &mut HashSet<(String, String)>, log: &mut Vec<String>) {
    for t in msg {
        let stamp = if stamp_static { Stamp::Static } else { Stamp::At(ros_time_to_timestamp(&t.stamp)) };
        match ros_to_transform(&t, stamp) {
            Ok(tr) => match reg.add_transform(tr) {
                Ok(_) => {}
                Err(e @ RegistryError::ReparentingNotSupported { .. }) => {
                    if conflicts.insert((t.frame_id.clone(), t.child.clone())) {
                        log.push(format!("WARN Ignoring transform {} -> {}: {e}. The first publisher of that frame wins.", t.frame_id, t.child));
                    } else {
                        log.push(format!("TRACE Ignoring transform {} -> {}: {e}", t.frame_id, t.child));
                    }
                }
                Err(e) => log.push(format!("ERROR Failed to add transform {} -> {}: {}", t.frame_id, t.child, e)),
            },
            Err(e) => log.push(format!("ERROR Skipping invalid transform {} -> {}: {}", t.frame_id, t.child, e)),
        }
    }
}
fn add_r2r(reg: &mut Registry<Timestamp>, msg: Vec<Tf>, stamp_static: bool, log: &mut Vec<String>) {
    for t in msg {
        let stamp = if stamp_static { Stamp::Static } else { Stamp::At(ros_time_to_timestamp(&t.stamp)) };
        match ros_to_transform(&t, stamp) {
            Ok(tr) => {
                if let Err(e) = reg.add_transform(tr) {
                    log.push(format!("ERROR Failed to add transform {} -> {}: {}", t.frame_id, t.child, e));
                }
            }
            Err(e) => log.push(format!("ERROR Skipping invalid transform {} -> {}: {}", t.frame_id, t.child, e)),
        }
    }
}
/// returns (x, "parent<-child", header stamp)
fn lookup_mt(reg: &Registry<Timestamp>, target_frame: &str, source_frame: &str, time: Time) -> Result<(f64, String, Time), String> {
    let timestamp = ros_time_to_timestamp(&time);
    let ts = if time.sec == 0 && time.nanosec == 0 { Timestamp::now() } else { timestamp };
    let t = reg.get_transform(target_frame, source_frame, ts).map_err(|e| format!("Lookup({e})"))?;
    Ok((t.translation().x, format!("{}<-{}", t.parent(), t.child()), time))
}
fn lookup_r2r(reg: &Registry<Timestamp>, source_frame: &str, target_frame: &str, time: Time) -> Result<(f64, String), String> {
    let timestamp = ros_time_to_timestamp(&time);
    let ts = if time.sec == 0 && time.nanosec == 0 { Timestamp::now() } else { timestamp };
    let t = reg.get_transform(source_frame, target_frame, ts).map_err(|e| format!("Lookup({e})"))?;
    Ok((t.translation().x, format!("{}<-{}", t.parent(), t.child())))
}
fn reg10() -> Registry<Timestamp> {
    Registry::with_max_age(Duration::from_secs(10))
}
const Z: Time = Time { sec: 0, nanosec: 0 };

fn main() {
    // ---------------- V1: time zero -> now()
    println!("== V1 time zero");
    {
        let mut r = reg10();
        let mut log = vec![];
        let now = Timestamp::now().as_nanos();
        for i in 0..10u64 {
            add_r2r(&mut r, vec![tf("map", "base", ts_from_ns(now - 100_000_000 + i * 10_000_000), 3.0)], false, &mut log);
        }
        println!("real-time stream newest 10ms old: {:?}", lookup_mt(&r, "map", "base", Z));
        let lct = r.latest_common_time("map", "base");
        println!("latest_common_time: {lct:?} -> {:?}", lct.as_ref().map(|s| r.get_transform("map", "base", s.at().unwrap()).map(|t| t.translation().x)));
        // publisher clock ahead of listener by 50 ms (cross-machine skew)
        let mut r2 = reg10();
        let now = Timestamp::now().as_nanos();
        for i in 0..11u64 {
            add_r2r(&mut r2, vec![tf("map", "base", ts_from_ns(now - 50_000_000 + i * 10_000_000), i as f64)], false, &mut log);
        }
        println!("skewed publisher (+50ms), zero-time lookup: {:?}  (newest sample x=10)", lookup_mt(&r2, "map", "base", Z));
        let mut r3 = reg10();
        for i in 0..=10 {
            add_r2r(&mut r3, vec![tf("odom", "base", secs(100.0 + i as f64 * 0.1), 1.0)], false, &mut log);
        }
        println!("sim time: {:?}", lookup_mt(&r3, "odom", "base", Z));
        add_r2r(&mut r3, vec![tf("base", "laser", Z, 0.3)], true, &mut log);
        println!("static chain: {:?}", lookup_mt(&r3, "base", "laser", Z));
        println!("logs {log:?}");
    }
    // ---------------- V2: reparent static
    println!("== V2 static parent change");
    {
        let mut r = reg10();
        let (mut c, mut log) = (HashSet::new(), vec![]);
        add_mt(&mut r, vec![tf("base_link", "laser", Z, 0.3)], true, &mut c, &mut log);
        for _ in 0..3 {
            add_mt(&mut r, vec![tf("mast", "laser", secs(5.0), 0.5)], true, &mut c, &mut log);
        }
        println!("logs: {log:?}");
        println!("base_link<-laser @50s {:?}", lookup_mt(&r, "base_link", "laser", secs(50.0)));
        println!("mast<-laser @50s {:?}", lookup_mt(&r, "mast", "laser", secs(50.0)));
        let before = lookup_mt(&r, "base_link", "laser", secs(10.0));
        add_mt(&mut r, vec![tf("base_link", "laser", Z, 0.35)], true, &mut c, &mut log);
        println!("same parent re-publish: before {before:?} after {:?}", lookup_mt(&r, "base_link", "laser", secs(10.0)));
    }
    // ---------------- V3: kind flavour
    println!("== V3 static then dynamic");
    {
        let mut r = reg10();
        let (mut c, mut log) = (HashSet::new(), vec![]);
        add_mt(&mut r, vec![tf("map", "odom", Z, 0.0)], true, &mut c, &mut log);
        for i in 0..5 {
            add_mt(&mut r, vec![tf("map", "odom", secs(100.0 + i as f64 * 0.5), 1.0 + i as f64 * 0.5)], false, &mut c, &mut log);
        }
        println!("{} log lines; first {:?}", log.len(), log.first());
        println!("map<-odom @102s (publisher said 3.0): {:?}", lookup_mt(&r, "map", "odom", secs(102.0)));
        let mut r = reg10();
        let mut log = vec![];
        for i in 0..3 {
            add_mt(&mut r, vec![tf("map", "odom", secs(100.0 + i as f64 * 0.5), 1.0)], false, &mut c, &mut log);
        }
        add_mt(&mut r, vec![tf("map", "odom", Z, 0.0)], true, &mut c, &mut log);
        println!("reverse: logs {log:?}; @200s {:?}", lookup_mt(&r, "map", "odom", secs(200.0)));
    }
    // ---------------- V4: empty frame ids
    println!("== V4 empty frame_id");
    {
        let mut r = reg10();
        let (mut c, mut log) = (HashSet::new(), vec![]);
        add_mt(&mut r, vec![tf("", "camera", Z, 1.0), tf("", "lidar", Z, 3.0)], true, &mut c, &mut log);
        println!("logs {log:?}; camera<-lidar {:?}", lookup_mt(&r, "camera", "lidar", secs(1.0)));
        // dynamic, empty child too
        let mut log = vec![];
        add_r2r(&mut r, vec![tf("map", "", secs(1.0), 5.0)], false, &mut log);
        println!("empty child on /tf: logs {log:?}");
    }
    // ---------------- V5: negative seconds
    println!("== V5 negative seconds");
    {
        for t in [Time { sec: -1, nanosec: 500_000_000 }, Time { sec: 0, nanosec: 500_000_000 }, Time { sec: -7, nanosec: 500_000_000 }] {
            println!("{t:?} -> {} ns", ros_time_to_timestamp(&t).as_nanos());
        }
        let mut r = reg10();
        let mut log = vec![];
        add_r2r(&mut r, vec![tf("odom", "base", Time { sec: -1, nanosec: 500_000_000 }, 1.0)], false, &mut log);
        add_r2r(&mut r, vec![tf("odom", "base", Time { sec: 0, nanosec: 500_000_000 }, 2.0)], false, &mut log);
        println!("logs {log:?}; lookup @(-1,0.5e9): {:?}", lookup_mt(&r, "odom", "base", Time { sec: -1, nanosec: 500_000_000 }));
        // (-1, 0) maps to 0 ns: is it treated as 'latest'? (no: zero check is on raw fields)
        println!("(-1,0) -> {} ns; zero-check fires? {}", ros_time_to_timestamp(&Time { sec: -1, nanosec: 0 }).as_nanos(), false);
    }
    // ---------------- V6: clock jump under max_age 10 s
    println!("== V6 clock jump");
    {
        let mut r = reg10();
        let mut log = vec![];
        for i in 0..=300 {
            let s = i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), s)], false, &mut log);
        }
        let n_before = log.len();
        for i in 0..=50 {
            let s = i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), -10.0)], false, &mut log);
        }
        println!("reset: {} new log lines", log.len() - n_before);
        println!("lookup @2.5s: {:?}", lookup_mt(&r, "odom", "base", secs(2.5)));
        println!("latest_common_time: {:?}", r.latest_common_time("odom", "base"));
        // small jump
        let mut r = reg10();
        for i in 0..=50 {
            let s = i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), s)], false, &mut log);
        }
        for i in 0..10 {
            let s = 2.55 + i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), -10.0)], false, &mut log);
        }
        println!("small jump lookup @2.52 (old 2.52, new -10): {:?}", lookup_mt(&r, "odom", "base", secs(2.52)));
        // remove_transforms_before recovery on the raw registry
        let mut r = reg10();
        for i in 0..=300 {
            let s = i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), s)], false, &mut log);
        }
        r.remove_transforms_before(Timestamp::from_nanos(u64::MAX));
        for i in 0..=50 {
            let s = i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), -10.0)], false, &mut log);
        }
        println!("after remove_transforms_before(MAX) recovery: @2.5s {:?}", lookup_mt(&r, "odom", "base", secs(2.5)));
    }
    // ---------------- V7: quaternions
    println!("== V7 quaternion tolerance");
    for (name, q) in [
        ("0.7071/0.7071", [0.7071, 0.0, 0.0, 0.7071]),
        ("0.707/0.707", [0.707, 0.0, 0.0, 0.707]),
        ("0.70710678", [0.70710678, 0.0, 0.0, 0.70710678]),
        ("f32 widened", [std::f32::consts::FRAC_1_SQRT_2 as f64, 0.0, 0.0, std::f32::consts::FRAC_1_SQRT_2 as f64]),
    ] {
        let n2: f64 = q.iter().map(|v| v * v).sum();
        let tf2_ok = (n2 - 1.0).abs() < 10e-3;
        let mut t = tf("base", "cam", Z, 0.0);
        t.q = q;
        println!("{name}: norm-1 {:.3e}, tf2 accepts {tf2_ok}, transforms {:?}", n2.sqrt() - 1.0, ros_to_transform(&t, Stamp::Static).map(|_| ()));
    }
    // ---------------- V8: parameter names
    println!("== V8 param names");
    {
        let mut r = reg10();
        let (mut c, mut log) = (HashSet::new(), vec![]);
        add_mt(&mut r, vec![tf("base_link", "laser", Z, 0.3)], true, &mut c, &mut log);
        // A caller who wants laser data expressed in base_link, binding by the declared names:
        let source_frame = "laser";
        let target_frame = "base_link";
        println!("r2r lookup_transform(source_frame, target_frame): {:?}", lookup_r2r(&r, source_frame, target_frame, secs(1.0)));
        println!("mt  lookup_transform(target_frame, source_frame): {:?}", lookup_mt(&r, target_frame, source_frame, secs(1.0)));
    }
    // ---------------- V9: gap interpolation
    println!("== V9 dropout");
    {
        let mut r = reg10();
        let mut log = vec![];
        add_r2r(&mut r, vec![tf("odom", "base", secs(100.0), 0.0)], false, &mut log);
        add_r2r(&mut r, vec![tf("odom", "base", secs(108.0), 8.0)], false, &mut log);
        println!("@104 {:?}", lookup_mt(&r, "odom", "base", secs(104.0)));
    }
    // ---------------- V10: static log replay
    println!("== V10 static log replay");
    {
        let mut live = reg10();
        let (mut c, mut log) = (HashSet::new(), vec![]);
        let mut slog: BTreeMap<(String, String), Tf> = BTreeMap::new();
        let msgs = vec![
            vec![tf("base", "z_mount", Z, 0.0), tf("base", "a_mount", Z, 0.0)],
            vec![tf("z_mount", "laser", Z, 1.0)],
            vec![tf("a_mount", "laser", Z, 2.0)],
        ];
        for m in msgs {
            for t in &m {
                slog.insert((t.frame_id.clone(), t.child.clone()), t.clone());
            }
            add_mt(&mut live, m, true, &mut c, &mut log);
        }
        let replayed: Vec<Tf> = slog.values().cloned().collect();
        let n = replayed.len();
        let mut rep = reg10();
        let (mut c2, mut log2) = (HashSet::new(), vec![]);
        add_mt(&mut rep, replayed, true, &mut c2, &mut log2);
        println!("entries {n}; live {:?}; replay {:?}; replay logs {log2:?}", lookup_mt(&live, "base", "laser", secs(1.0)), lookup_mt(&rep, "base", "laser", secs(1.0)));
    }
    // ---------------- N1: dynamic parent conflict dedupe, first publisher stops
    println!("== N1 dynamic conflict dedupe");
    {
        let mut r = reg10();
        let (mut c, mut log) = (HashSet::new(), vec![]);
        for i in 0..10 {
            add_mt(&mut r, vec![tf("map_a", "odom", secs(100.0 + i as f64 * 0.1), 1.0)], false, &mut c, &mut log);
        }
        // publisher A stops; publisher B takes over with another parent
        for i in 10..60 {
            add_mt(&mut r, vec![tf("map_b", "odom", secs(100.0 + i as f64 * 0.1), 2.0)], false, &mut c, &mut log);
        }
        let warns = log.iter().filter(|l| l.starts_with("WARN")).count();
        let traces = log.iter().filter(|l| l.starts_with("TRACE")).count();
        println!("warn {warns} trace {traces}; map_a<-odom @105 {:?}; map_b<-odom @105 {:?}", lookup_mt(&r, "map_a", "odom", secs(105.0)), lookup_mt(&r, "map_b", "odom", secs(105.0)));
    }
    // ---------------- N2: zero-stamped /tf sample inside a live max_age stream
    println!("== N2 zero-stamped /tf sample");
    {
        let mut r = reg10();
        let mut log = vec![];
        for i in 0..=10 {
            add_r2r(&mut r, vec![tf("odom", "base", secs(100.0 + i as f64 * 0.1), 1.0)], false, &mut log);
        }
        add_r2r(&mut r, vec![tf("odom", "base", Z, 9.0)], false, &mut log);
        println!("logs {log:?}; lookup @(0,1ns) {:?}", lookup_mt(&r, "odom", "base", Time { sec: 0, nanosec: 1 }));
    }

    // ---------------- N3: large backward jump: does the loss stay permanent?
    println!("== N3 large jump, new epoch keeps running");
    {
        let mut r = reg10();
        let mut log = vec![];
        for i in 0..=300 {
            let s = i as f64 * 0.1;
            add_r2r(&mut r, vec![tf("odom", "base", secs(s), s)], false, &mut log);
        }
        let probe_at = |r: &Registry<Timestamp>, upto: f64| {
            println!("  new epoch up to {upto}s: lookup @{:.1} {:?}; @25.05 {:?}; lct {:?}", upto - 0.05, lookup_mt(r, "odom", "base", secs(upto - 0.05)).map(|v| v.0), lookup_mt(r, "odom", "base", secs(25.05)).map(|v| v.0), r.latest_common_time("odom", "base").map(|s| s.at().unwrap().as_seconds_lossy()));
        };
        let mut t = 0.0;
        for stop in [5.0, 19.9, 25.0, 31.0, 41.0] {
            while t <= stop + 1e-9 {
                add_r2r(&mut r, vec![tf("odom", "base", secs(t), -10.0)], false, &mut log);
                t += 0.1;
            }
            probe_at(&r, stop);
        }
        println!("  log lines {}", log.len());
    }
    // ---------------- N4: post-dated hop (nav2 AMCL stamps map->odom at scan time + transform_tolerance, default 1.0 s)
    println!("== N4 post-dated map->odom, zero-time lookup");
    {
        let mut r = reg10();
        let mut log = vec![];
        let now = Timestamp::now().as_nanos();
        for i in 0..100u64 {
            // scans every 50 ms, newest 10 ms ago, stamped +1 s
            let scan = now - 4_960_000_000 + i * 50_000_000;
            add_r2r(&mut r, vec![tf("map", "odom", ts_from_ns(scan + 1_000_000_000), i as f64)], false, &mut log);
            add_r2r(&mut r, vec![tf("odom", "base_link", ts_from_ns(scan), 0.0)], false, &mut log);
        }
        println!("map<-odom zero-time: {:?} (newest sample x=99)", lookup_mt(&r, "map", "odom", Z));
        println!("map<-base_link zero-time: {:?}", lookup_mt(&r, "map", "base_link", Z).map(|v| v.0));
    }
}
