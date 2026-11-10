// Verifier probe. Depends only on transforms (this checkout).
// Mirrors roslibrust_transforms 0.26.0 src/lib.rs (cited per block).
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use transforms::geometry::{Quaternion, Vector3};
use transforms::time::{Stamp, Timestamp};
use transforms::{Registry, Transform};

// lib.rs:90-106 (verbatim logic)
fn ts_from_ros_time(sec: i32, nsec: u32) -> Timestamp {
    match u64::try_from(sec) {
        Ok(sec) => Timestamp::from_nanos(sec * 1_000_000_000 + (nsec as u64)),
        Err(_) => Timestamp::zero(),
    }
}
fn ts_as_ros_time(t: Timestamp) -> (i32, u32) {
    let nanos = t.as_nanos();
    ((nanos / 1_000_000_000) as i32, (nanos % 1_000_000_000) as u32)
}
// lib.rs:108-127 (verbatim logic)
fn st_from_ros_time(sec: i32, nsec: u32) -> SystemTime {
    if sec >= 0 {
        UNIX_EPOCH + Duration::new(sec as u64, nsec)
    } else {
        UNIX_EPOCH
            .checked_sub(Duration::new((-sec) as u64, nsec))
            .unwrap_or(UNIX_EPOCH)
    }
}
fn st_as_ros_time(t: SystemTime) -> (i32, u32) {
    let d = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    (d.as_secs() as i32, d.subsec_nanos())
}

// Stand-in for Ros2TransformStamped fields.
struct Msg {
    frame_id: String,
    child: String,
    sec: i32,
    nanosec: u32,
    t: (f64, f64, f64),
    q: (f64, f64, f64, f64), // x,y,z,w as on the wire
}
fn msg(p: &str, c: &str, sec: i32, nanosec: u32, x: f64) -> Msg {
    Msg { frame_id: p.into(), child: c.into(), sec, nanosec, t: (x, 0.0, 0.0), q: (0.0, 0.0, 0.0, 1.0) }
}

// lib.rs:785-815 ROS2 into_transform
fn into_transform(m: Msg, is_static: bool) -> Result<Transform, transforms::errors::TransformError> {
    let timestamp = if is_static { Stamp::Static } else { Stamp::At(ts_from_ros_time(m.sec, m.nanosec)) };
    Transform::new(
        &m.frame_id,
        &m.child,
        Vector3::new(m.t.0, m.t.1, m.t.2),
        Quaternion { w: m.q.3, x: m.q.0, y: m.q.1, z: m.q.2 },
        timestamp,
    )
}

// lib.rs:310-333 ingest loop body
fn ingest(reg: &mut Registry, msgs: Vec<Msg>, is_static: bool) {
    let topic = if is_static { "/tf_static" } else { "/tf" };
    for tf in msgs {
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
            println!("    WARN Dropping transform from '{parent}' to '{child}' rejected by the registry on {topic}: {e}");
        }
    }
}

fn mgr() -> Registry {
    Registry::with_max_age(Duration::from_secs(10)) // lib.rs:230
}
fn s(x: f64) -> Timestamp {
    Timestamp::from_nanos((x * 1e9).round() as u64)
}
fn show(r: Result<Transform, transforms::errors::RegistryError>) -> String {
    match r {
        Ok(t) => format!("Ok(x={:.6} parent={} child={} stamp={:?})", t.translation().x, t.parent(), t.child(), t.timestamp()),
        Err(e) => format!("Err({e}) [{e:?}]"),
    }
}

fn main() {
    println!("V1 empty frame ids (Z1)");
    let mut r = mgr();
    ingest(&mut r, vec![msg("", "camera", 5, 0, 1.0), msg("", "lidar", 5, 0, 3.0)], false);
    println!("  camera<-lidar at 5s: {}", show(r.get_transform("camera", "lidar", s(5.0))));

    println!("V2 static-stale (Z2)");
    let mut r = mgr();
    ingest(&mut r, vec![msg("base", "arm", 0, 0, 0.5)], true);
    ingest(&mut r, vec![msg("base", "arm", 10, 0, 1.0)], false);
    ingest(&mut r, vec![msg("base", "arm", 11, 0, 2.0)], false);
    println!("  base<-arm at 10.5s: {}", show(r.get_transform("base", "arm", s(10.5))));
    println!("  latest_common_time: {:?}", r.latest_common_time("base", "arm"));
    // Reverse order: dynamic first, then static on /tf_static
    let mut r = mgr();
    ingest(&mut r, vec![msg("base", "arm", 10, 0, 1.0)], false);
    ingest(&mut r, vec![msg("base", "arm", 0, 0, 0.5)], true);
    println!("  reverse order (dynamic first) latest: {:?}", r.latest_common_time("base", "arm"));

    println!("V3a clock reset > max_age (Z3)");
    let mut r = mgr();
    for i in 100..=105 { ingest(&mut r, vec![msg("odom", "base", i, 0, i as f64)], false); }
    for i in 0..=3 {
        let tf = into_transform(msg("odom", "base", i, 0, i as f64), false).unwrap();
        println!("    insert {i}s -> {:?}", r.add_transform(tf));
    }
    let l = r.latest_common_time("odom", "base");
    println!("  latest_common_time: {l:?}");
    if let Ok(Stamp::At(t)) = l { println!("  get at latest: {}", show(r.get_transform("odom", "base", t))); }
    println!("  get at 2s: {}", show(r.get_transform("odom", "base", s(2.0))));

    println!("V3b clock reset < max_age (Z3)");
    let mut r = mgr();
    for i in 0..=8 { ingest(&mut r, vec![msg("odom", "base", i, 0, 10.0 * i as f64)], false); }
    for (sec, ns) in [(0, 500_000_000u32), (1, 500_000_000), (2, 500_000_000)] { ingest(&mut r, vec![msg("odom", "base", sec, ns, 0.0)], false); }
    for t in [2.0, 2.2, 3.0] { println!("  at {t}s: {}", show(r.get_transform("odom", "base", s(t)))); }
    println!("  latest: {:?}", r.latest_common_time("odom", "base"));

    println!("V4 quaternions (Z5)");
    let f = |x: f64, y: f64, z: f64, w: f64| {
        let n2 = x * x + y * y + z * z + w * w;
        let tf2 = (n2 - 1.0).abs() < 10e-3;
        let r = Transform::new("a", "b", Vector3::zero(), Quaternion { w, x, y, z }, Stamp::At(s(1.0)));
        format!("|q|-1={:.3e} tf2_accepts={tf2} transforms={:?}", n2.sqrt() - 1.0, r.map(|_| ()))
    };
    println!("  (0,0,.7071,.7071) {}", f(0.0, 0.0, 0.7071, 0.7071));
    println!("  (0,0,.707,.707)   {}", f(0.0, 0.0, 0.707, 0.707));
    let (sz, cz) = ((std::f64::consts::PI / 8.0).sin() as f32 as f64, (std::f64::consts::PI / 8.0).cos() as f32 as f64);
    println!("  f32 yaw45         {}", f(0.0, 0.0, sz, cz));
    println!("  (0,0,.7071,.7071) normalized -> {:?}", Quaternion { w: 0.7071, x: 0.0, y: 0.0, z: 0.7071 }.normalize().map(|q| Transform::new("a", "b", Vector3::zero(), q, Stamp::At(s(1.0))).map(|_| ())));

    println!("V5 cross-time re-insert (Z4)");
    let mut r = mgr();
    let (t1, t2) = (s(1.0), s(2.0));
    r.add_transform(Transform::new("map", "odom", Vector3::new(0.0, 0.0, 0.0), Quaternion::identity(), Stamp::At(t1)).unwrap()).unwrap();
    r.add_transform(Transform::new("map", "odom", Vector3::new(4.0, 0.0, 0.0), Quaternion::identity(), Stamp::At(t2)).unwrap()).unwrap();
    r.add_transform(Transform::new("odom", "base", Vector3::new(1.0, 0.0, 0.0), Quaternion::identity(), Stamp::At(t1)).unwrap()).unwrap();
    r.add_transform(Transform::new("odom", "base", Vector3::new(5.0, 0.0, 0.0), Quaternion::identity(), Stamp::At(t2)).unwrap()).unwrap();
    println!("  odom<-base at t2 before: {}", show(r.get_transform("odom", "base", t2)));
    let ct = r.get_transform_at("odom", t2, "base", t1, "map").unwrap();
    println!("  get_transform_at(odom,t2,base,t1,map): {}", show(Ok(ct.clone())));
    // lib.rs:518-547: registry first, then publish on /tf with as_ros_time stamp
    let wire = match ct.timestamp() { Stamp::At(t) => ts_as_ros_time(t), Stamp::Static => (0, 0) };
    let topic = if ct.timestamp().is_static() { "/tf_static" } else { "/tf" };
    println!("  manager.add_transform -> {:?}; would publish on {topic} stamp {wire:?}", r.add_transform(ct));
    println!("  odom<-base at t2 after: {}", show(r.get_transform("odom", "base", t2)));

    println!("V6 time edges (Z6)");
    println!("  from_ros_time(-1,5e8)={:?} (-7,0)={:?}", ts_from_ros_time(-1, 500_000_000), ts_from_ros_time(-7, 0));
    let mut r = mgr();
    ingest(&mut r, vec![msg("odom", "base", -1, 500_000_000, 0.0), msg("odom", "base", -7, 0, 1.0)], false);
    println!("  lookup at 0: {}", show(r.get_transform("odom", "base", Timestamp::zero())));
    // lib.rs:691-694 ROS1: nsecs (i32) as u32
    let ros1_nsecs: i32 = -1;
    println!("  ROS1 (10,-1) -> {:?} s", ts_from_ros_time(10, ros1_nsecs as u32).as_nanos() as f64 / 1e9);
    let st = st_from_ros_time(-2, 500_000_000);
    println!("  SystemTime(-2,+5e8) -> {:?} before epoch", UNIX_EPOCH.duration_since(st));
    println!("  SystemTime pre-epoch as_ros_time -> {:?}", st_as_ros_time(st));
    let p = std::panic::catch_unwind(|| st_from_ros_time(i32::MIN, 0));
    println!("  SystemTime(i32::MIN,0) panicked={}", p.is_err());
    println!("  Timestamp(i32::MIN,0) -> {:?}", ts_from_ros_time(i32::MIN, 0));
    println!("  as_ros_time(2.2e9 s) = {:?}", ts_as_ros_time(Timestamp::from_nanos(2_200_000_000 * 1_000_000_000)));

    println!("V7 stringified error; permanent NotFoundAt (Z7)");
    let mut r = mgr();
    for i in 101..=111 { ingest(&mut r, vec![msg("odom", "base", i, 0, 1.0)], false); }
    let e = r.get_transform("odom", "base", s(50.0)).unwrap_err();
    println!("  typed: {e:?}");
    println!("  LookupError(\"{e}\")");
    let late = into_transform(msg("odom", "base", 50, 0, 1.0), false).unwrap();
    println!("  late sample at 50s insert -> {:?}; lookup at 50s after: {}", r.add_transform(late), show(r.get_transform("odom", "base", s(50.0))));

    println!("V8 remove-then-add pre-emption (S3)");
    let mut r = mgr();
    ingest(&mut r, vec![msg("world", "tool", 1, 0, 1.0)], false);
    println!("  remove_frame(tool) = {}", r.remove_frame("tool"));
    ingest(&mut r, vec![msg("world", "tool", 2, 0, 1.0)], false); // late message from old parent
    let g = Transform::new("gripper", "tool", Vector3::zero(), Quaternion::identity(), Stamp::At(s(3.0))).unwrap();
    println!("  add_transform(gripper->tool) = {:?}", r.add_transform(g));

    println!("V9 lookup at now-ish (Z9)");
    let mut r = mgr();
    ingest(&mut r, vec![msg("map", "base_link", 100, 0, 1.0)], false);
    println!("  at 100.02s: {}", show(r.get_transform("map", "base_link", s(100.02))));

    println!("V10 gap interpolation (Z8)");
    let mut r = mgr();
    ingest(&mut r, vec![msg("odom", "base", 100, 0, 0.0), msg("odom", "base", 108, 0, 8.0)], false);
    println!("  at 104s: {}", show(r.get_transform("odom", "base", s(104.0))));

    println!("VX extra: insert errors' Display (F1)");
    let mut r = mgr();
    r.add_transform(Transform::new("a", "b", Vector3::zero(), Quaternion::identity(), Stamp::At(s(1.0))).unwrap()).unwrap();
    let e1 = r.add_transform(Transform::static_between("a", "b", Vector3::zero(), Quaternion::identity()).unwrap()).unwrap_err();
    let e2 = r.add_transform(Transform::new("c", "b", Vector3::zero(), Quaternion::identity(), Stamp::At(s(1.0))).unwrap()).unwrap_err();
    let e3 = r.add_transform(Transform::new("b", "a", Vector3::zero(), Quaternion::identity(), Stamp::At(s(1.0))).unwrap()).unwrap_err();
    println!("  {e1:?} | {e2:?} | {e3:?}");
}
