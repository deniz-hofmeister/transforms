//! Probes mirroring roslibrust_transforms 0.26.0 (== upstream master a798361).
//! Every mirrored function cites the lines of roslibrust_transforms/src/lib.rs
//! it copies. Only `transforms` (path dep on this checkout, read at f9770d2)
//! is linked; no roslibrust code is built or run.

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use transforms::{
    Registry,
    errors::TransformError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

// ---- mirror of lib.rs:90-106 (RosTimestamp for Timestamp) ----
fn from_ros_time(sec: i32, nsec: u32) -> Timestamp {
    match u64::try_from(sec) {
        Ok(sec) => Timestamp::from_nanos(sec * 1_000_000_000 + (nsec as u64)),
        Err(_) => Timestamp::zero(),
    }
}
fn as_ros_time(t: Timestamp) -> (i32, u32) {
    let nanos = t.as_nanos();
    let secs = nanos / 1_000_000_000;
    let nsecs = nanos % 1_000_000_000;
    (secs as i32, nsecs as u32)
}

// ---- mirror of lib.rs:108-119 (RosTimestamp::from_ros_time for SystemTime) ----
fn systime_from_ros_time(sec: i32, nsec: u32) -> SystemTime {
    if sec >= 0 {
        UNIX_EPOCH + Duration::new(sec as u64, nsec)
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::new((-sec) as u64, nsec))
            .unwrap_or(UNIX_EPOCH)
    }
}

/// A TransformStamped as the ROS2 codegen delivers it (messages.rs: Header
/// { stamp: builtin_interfaces::Time { sec: i32, nanosec: u32 }, frame_id }).
#[derive(Clone)]
struct Msg {
    frame_id: &'static str,
    child: &'static str,
    sec: i32,
    nanosec: u32,
    t: [f64; 3],
    q: [f64; 4], // x, y, z, w as on the wire
}

// ---- mirror of lib.rs:785-815 (IntoTransform for Ros2TransformStamped) ----
fn into_transform(m: Msg, is_static: bool) -> Result<Transform, TransformError> {
    let timestamp = if is_static {
        Stamp::Static
    } else {
        Stamp::At(from_ros_time(m.sec, m.nanosec))
    };
    Transform::new(
        m.frame_id,
        m.child,
        Vector3::new(m.t[0], m.t[1], m.t[2]),
        Quaternion { w: m.q[3], x: m.q[0], y: m.q[1], z: m.q[2] },
        timestamp,
    )
}

// ---- mirror of lib.rs:310-333 (process_tf_messages body for one message) ----
fn ingest(reg: &mut Registry, msg: Vec<Msg>, is_static: bool) {
    let topic = if is_static { "/tf_static" } else { "/tf" };
    for tf in msg {
        let transform = match into_transform(tf, is_static) {
            Ok(t) => t,
            Err(e) => {
                println!("    WARN Dropping invalid transform received on {topic}: {e}");
                continue;
            }
        };
        let parent = transform.parent().to_owned();
        let child = transform.child().to_owned();
        if let Err(e) = reg.add_transform(transform) {
            println!(
                "    WARN Dropping transform from '{parent}' to '{child}' rejected by the registry on {topic}: {e}"
            );
        }
    }
}

const ID: [f64; 4] = [0.0, 0.0, 0.0, 1.0];
fn m(frame_id: &'static str, child: &'static str, sec: i32, nanosec: u32, x: f64) -> Msg {
    Msg { frame_id, child, sec, nanosec, t: [x, 0.0, 0.0], q: ID }
}
fn ts(secs: f64) -> Timestamp {
    Timestamp::from_nanos((secs * 1e9).round() as u64)
}
fn x_of(r: Result<Transform, impl std::fmt::Display>) -> String {
    match r {
        Ok(t) => format!("Ok(x={})", t.translation().x),
        Err(e) => format!("Err({e})"),
    }
}

fn main() {
    // lib.rs:230 — the manager's registry; 10 s is the value every example/doc uses
    let buffer = Duration::from_secs(10);

    println!("P1 empty frame_id (ROS2 default for an unset string field)");
    {
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("", "camera", 5, 0, 1.0)], false);
        ingest(&mut reg, vec![m("", "lidar", 5, 0, 3.0)], false);
        println!("  camera->lidar at 5s: {}", x_of(reg.get_transform("camera", "lidar", ts(5.0))));
    }

    println!("P2 static-stale trap: frame first on /tf_static, later moves to /tf");
    {
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("base", "arm", 0, 0, 0.5)], true);
        ingest(&mut reg, vec![m("base", "arm", 10, 0, 1.0)], false);
        ingest(&mut reg, vec![m("base", "arm", 11, 0, 2.0)], false);
        println!("  base<-arm at 10.5s: {}", x_of(reg.get_transform("base", "arm", ts(10.5))));
        println!("  latest_common_time(base, arm): {:?}", reg.latest_common_time("base", "arm"));
    }

    println!("P3a clock reset larger than max_age (sim restart 105s -> 0s)");
    {
        let mut reg = Registry::with_max_age(buffer);
        for s in 100..=105 {
            ingest(&mut reg, vec![m("odom", "base", s, 0, s as f64)], false);
        }
        for s in 0..=3 {
            ingest(&mut reg, vec![m("odom", "base", s, 0, 0.0)], false);
        }
        let latest = reg.latest_common_time("odom", "base");
        println!("  latest_common_time after reset: {latest:?}");
        if let Ok(Stamp::At(t)) = latest {
            println!("  get_transform at latest: {}", x_of(reg.get_transform("odom", "base", t)));
        }
        println!("  get_transform at new 2s: {}", x_of(reg.get_transform("odom", "base", ts(2.0))));
    }

    println!("P3b clock reset smaller than max_age (run1 0..8s moving x=10t, run2 at origin)");
    {
        let mut reg = Registry::with_max_age(buffer);
        for s in 0..=8 {
            ingest(&mut reg, vec![m("odom", "base", s, 0, 10.0 * s as f64)], false);
        }
        for half in [0.5, 1.5, 2.5] {
            let sec = half as i32;
            ingest(&mut reg, vec![m("odom", "base", sec, 500_000_000, 0.0)], false);
        }
        for q in [2.0, 2.2, 3.0] {
            println!("  run2 lookup at {q}s: {}", x_of(reg.get_transform("odom", "base", ts(q))));
        }
        println!("  latest_common_time: {:?}", reg.latest_common_time("odom", "base"));
    }

    println!("P4 quaternion tolerance vs tf2 (|q|^2-1 < 10e-3, buffer_core.cpp)");
    {
        for (label, q) in [
            ("hand-typed 4 digits (0,0,0.7071,0.7071)", [0.0, 0.0, 0.7071, 0.7071]),
            ("3 digits (0,0,0.707,0.707)", [0.0, 0.0, 0.707, 0.707]),
            (
                "f32-rounded yaw 45deg",
                [0.0, 0.0, (std::f64::consts::FRAC_PI_8.sin() as f32) as f64, (std::f64::consts::FRAC_PI_8.cos() as f32) as f64],
            ),
        ] {
            let n2: f64 = q.iter().map(|v| v * v).sum();
            let tf2_ok = (n2 - 1.0).abs() < 10e-3;
            let msg = Msg { frame_id: "base", child: "cam", sec: 1, nanosec: 0, t: [0.0; 3], q };
            let ours = into_transform(msg, true).map(|_| ());
            println!("  {label}: |q|-1={:.3e} tf2_accepts={tf2_ok} roslibrust_transforms={ours:?}", n2.sqrt() - 1.0);
        }
        let q = Quaternion { w: 0.7071, x: 0.0, y: 0.0, z: 0.7071 };
        let n = q.normalize().unwrap();
        println!("  public Quaternion::normalize() then Transform::new: {:?}",
            Transform::<Timestamp>::static_between("base", "cam", Vector3::zero(), n).map(|_| ()));
    }

    println!("P5 get_transform_at result re-inserted via manager.add_transform (lib.rs:518-547)");
    {
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("map", "odom", 1, 0, 0.0), m("odom", "base", 1, 0, 0.0)], false);
        ingest(&mut reg, vec![m("map", "odom", 2, 0, 1.0), m("odom", "base", 2, 0, 5.0)], false);
        let (t1, t2) = (ts(1.0), ts(2.0));
        println!("  before: odom<-base at t2: {}", x_of(reg.get_transform("odom", "base", t2)));
        let x = reg.get_transform_at("odom", t2, "base", t1, "map").unwrap();
        println!("  get_transform_at(odom,t2,base,t1,map): parent={} child={} stamp={:?} x={}",
            x.parent(), x.child(), x.timestamp(), x.translation().x);
        let wire = match x.timestamp() { Stamp::At(t) => as_ros_time(t), Stamp::Static => (0, 0) };
        println!("  stamp FromTransform would publish on /tf (lib.rs:826-829): {wire:?}");
        println!("  add_transform(result): {:?}", reg.add_transform(x));
        println!("  after: odom<-base at t2: {}", x_of(reg.get_transform("odom", "base", t2)));
    }

    println!("P6 ROS time conversions");
    {
        println!("  from_ros_time(-1, 5e8) = {:?}; from_ros_time(-7, 0) = {:?} (collide)",
            from_ros_time(-1, 500_000_000), from_ros_time(-7, 0));
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("w", "a", -7, 0, 7.0)], false);
        ingest(&mut reg, vec![m("w", "a", -1, 500_000_000, 1.0)], false);
        println!("  two pre-epoch samples then lookup at t=0: {}", x_of(reg.get_transform("w", "a", Timestamp::zero())));
        // ROS1 path, lib.rs:691-694: nsecs is i32 in roslibrust's integral Time and is cast `as u32`
        let ros1_nsecs: i32 = -1;
        println!("  ROS1 (secs=10, nsecs=-1) -> {:?} s",
            from_ros_time(10, ros1_nsecs as u32).as_nanos() as f64 / 1e9);
        let st = systime_from_ros_time(-2, 500_000_000);
        println!("  SystemTime from (-2 s, +0.5 s) = -{:?} before epoch (ROS meaning: -1.5 s)",
            UNIX_EPOCH.duration_since(st).unwrap());
        let r = std::panic::catch_unwind(|| systime_from_ros_time(i32::MIN, 0));
        println!("  SystemTime from (i32::MIN, 0) in a debug build: {}",
            if r.is_err() { "PANIC (negation overflow)" } else { "no panic" });
        let after_2038 = Timestamp::from_nanos(2_200_000_000u64 * 1_000_000_000);
        println!("  as_ros_time(2.2e9 s, year 2039) = {:?}", as_ros_time(after_2038));
    }

    println!("P7 error stringification (lib.rs:399-402) and wait_for_transform (lib.rs:468)");
    {
        let mut reg = Registry::with_max_age(buffer);
        for s in 100..=111 {
            ingest(&mut reg, vec![m("odom", "base", s, 0, s as f64)], false);
        }
        let e = reg.get_transform("odom", "base", ts(50.0)).unwrap_err();
        println!("  typed: {e:?}");
        println!("  what a roslibrust caller gets: LookupError({:?})", e.to_string());
        println!("  (requested < covered start and max_age evicts anything older: wait_for_transform can only time out)");
    }

    println!("P8 remove_frame-then-add race (prior claim) vs reparent_frame");
    {
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("world", "tool", 1, 0, 1.0)], false);
        // user task: manager.remove_frame("tool") — lib.rs:640-643, own write guard
        println!("  remove_frame(tool) = {}", reg.remove_frame("tool"));
        // /tf task gets the guard next with one late old-parent message — lib.rs:310
        ingest(&mut reg, vec![m("world", "tool", 2, 0, 1.0)], false);
        // user task: manager.add_transform(gripper->tool) — lib.rs:530-533
        let new = into_transform(m("gripper", "tool", 2, 0, 0.0), false).unwrap();
        println!("  add_transform(gripper->tool) = {:?}", reg.add_transform(new.clone()));
        println!("  reparent_frame(gripper->tool) = {:?}", reg.reparent_frame(new));
        println!("  gripper<-tool at 2s: {}", x_of(reg.get_transform("gripper", "tool", ts(2.0))));
        // ... and the next old-parent message is rejected loudly
        ingest(&mut reg, vec![m("world", "tool", 3, 0, 1.0)], false);
    }
    println!("P9 README/doc idiom: get_transform(.., Timestamp::now()) on a dynamic chain");
    {
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("world", "base_link", 100, 0, 1.0)], false);
        // "now" = 20 ms after the newest /tf sample, as on any live system
        println!("  lookup 20 ms after newest sample: {}", x_of(reg.get_transform("world", "base_link", ts(100.02))));
    }

    println!("P10 publisher dropout inside max_age (10 s): interpolation across an 8 s gap");
    {
        let mut reg = Registry::with_max_age(buffer);
        ingest(&mut reg, vec![m("odom", "base", 100, 0, 0.0)], false);
        ingest(&mut reg, vec![m("odom", "base", 108, 0, 8.0)], false);
        println!("  lookup at 104 s: {}", x_of(reg.get_transform("odom", "base", ts(104.0))));
    }
}

