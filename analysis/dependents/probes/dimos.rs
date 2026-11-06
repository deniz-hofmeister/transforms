//! Mirrors dimos PR #4722 head 92936a7, native/rust/dimos-module/src/tf.rs,
//! against transforms 2.3.0 (master f9770d2, src unchanged since v2.3.0).
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use transforms::{
    Registry,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

// tf.rs:64-73 system_time(): signed ns -> SystemTime (pre-epoch allowed)
fn st(ns: i64) -> SystemTime {
    let d = Duration::from_nanos(ns.unsigned_abs());
    if ns >= 0 { UNIX_EPOCH.checked_add(d).unwrap() } else { UNIX_EPOCH.checked_sub(d).unwrap() }
}
fn q() -> Quaternion { Quaternion::identity() }
fn dyn_edge(p: &str, c: &str, ns: i64, x: f64) -> Transform<SystemTime> {
    Transform::new(p, c, Vector3::new(x, 0.0, 0.0), q(), Stamp::At(st(ns))).unwrap()
}
fn static_edge(p: &str, c: &str, x: f64) -> Transform<SystemTime> {
    Transform::new(p, c, Vector3::new(x, 0.0, 0.0), q(), Stamp::Static).unwrap()
}
// tf.rs:206-235 Graph::lookup(None): latest_common_time(..).at().unwrap_or(UNIX_EPOCH)
fn latest(r: &Registry<SystemTime>, t: &str, s: &str) -> Result<Transform<SystemTime>, String> {
    let time = r.latest_common_time(t, s).map_err(|e| e.to_string())?.at().unwrap_or(UNIX_EPOCH);
    let tf = r.get_transform(t, s, time).map_err(|e| e.to_string())?;
    // tf.rs:159-160 from_frame(): re-validates the lookup result
    tf.validate().map_err(|e| format!("from_frame validate: {e}"))?;
    Ok(tf)
}
fn ns_of(tf: &Transform<SystemTime>) -> i128 {
    let t = tf.timestamp().at().unwrap();
    match t.duration_since(UNIX_EPOCH) { Ok(d) => d.as_nanos() as i128, Err(e) => -(e.duration().as_nanos() as i128) }
}

fn main() {
    let w = Duration::from_secs(10); // DEFAULT_TF_WINDOW tf.rs:44

    // P1 mirrors test latest_resolves_the_whole_chain_at_its_common_time (tf.rs:457-477)
    let mut r = Registry::<SystemTime>::with_max_age(w);
    r.add_transforms([dyn_edge("map","odom",1_000_000_000,1.0), dyn_edge("map","odom",2_000_000_000,2.0), dyn_edge("odom","base",1_000_000_000,10.0)]).unwrap();
    let a = latest(&r, "map", "base").unwrap();
    let b = latest(&r, "base", "map").unwrap();
    println!("P1 latest map<-base x={} ns={} ; base<-map x={}", a.translation().x, ns_of(&a), b.translation().x);

    // P2 mirrors static_edges_survive_eviction_and_do_not_limit_dynamic_time (tf.rs:509-537)
    let mut r = Registry::<SystemTime>::with_max_age(w);
    r.add_transforms([static_edge("base","camera",3.0)]).unwrap();
    r.add_transforms([dyn_edge("map","base",1,1.0), dyn_edge("map","base",30_000_000_000,2.0)]).unwrap();
    let a = latest(&r, "map", "camera").unwrap();
    let neg = r.get_transform("base","camera", st(-1)).unwrap();
    println!("P2 map<-camera x={} ns={} ; base<-camera at -1ns x={} ns={}", a.translation().x, ns_of(&a), neg.translation().x, ns_of(&neg));

    // P3 mirrors invalid_batches_do_not_change_the_graph_or_publish (tf.rs:575-613), batch-cycle half
    let mut r = Registry::<SystemTime>::with_max_age(w);
    r.add_transforms([dyn_edge("map","base",1,1.0)]).unwrap();
    let e = r.add_transforms([dyn_edge("base","camera",1,3.0), dyn_edge("camera","map",1,4.0)]).unwrap_err();
    println!("P3 cycle batch err={e:?} ; base<-camera after = {:?}", latest(&r,"base","camera").map(|t| t.translation().x));
    // malformed rotation w=0 (tf.rs:596-603) is rejected by Transform::new before add_transforms
    let bad = Transform::new("base","camera", Vector3::zero(), Quaternion::from_wxyz(0.0,0.0,0.0,0.0), Stamp::At(st(1)));
    println!("P3b w=0 rotation -> {:?}", bad.err());

    // P4 pre-epoch stamps are a tested dimos path (tf.rs:539-546 ns=-1); what does a failure message print?
    let mut r = Registry::<SystemTime>::with_max_age(w);
    r.add_transforms([dyn_edge("map","base",-2_000_000_000,0.0), dyn_edge("map","base",-1_000_000_000,1.0)]).unwrap();
    println!("P4 pre-epoch miss: {}", r.get_transform("map","base", st(5)).unwrap_err());

    // P5 dimos isometry() checks |q.q - 1| <= 1e-6 (tf.rs:123-126); crate checks |‖q‖ - 1| <= 1e-6
    let n: f64 = 1.0 + 8e-7; // norm 1+8e-7, squared ~1+1.6e-6
    let qq = Quaternion::from_wxyz(n, 0.0, 0.0, 0.0);
    let sq = n * n;
    println!("P5 norm=1+8e-7: dimos squared check rejects={} ; crate Transform::new ok={}", (sq - 1.0).abs() > 1e-6,
        Transform::new("a","b",Vector3::zero(),qq,Stamp::At(st(1))).is_ok());

    // P6 re-validation of lookup output (tf.rs:160) vs the crate's not-re-validated derived results.
    // Ancestor-ward chain whose translations overflow to inf when composed.
    let mut r = Registry::<SystemTime>::new();
    r.add_transform(static_edge("map","a",1.5e308)).unwrap();
    r.add_transform(static_edge("a","b",1.5e308)).unwrap();
    let raw = r.get_transform("map","b", st(1));
    println!("P6 raw get_transform map<-b: {:?} ; dimos latest(): {:?}", raw.as_ref().map(|t| t.translation().x), latest(&r,"map","b").map(|t| t.translation().x));

    // P7 all-static chain: dimos stamps the wire result UNIX_EPOCH (= ROS time 0, which ROS reads as "latest")
    let mut r = Registry::<SystemTime>::new();
    r.add_transform(static_edge("base","camera",3.0)).unwrap();
    let a = latest(&r,"base","camera").unwrap();
    println!("P7 all-static latest ns={} lct={:?}", ns_of(&a), r.latest_common_time("base","camera").map(|s| s.is_static()));

    // P8 transforms_io 0.8.0 pattern: Time{0,0} -> Timestamp::now() (mt_pubsub_backend.rs:296-300)
    let mut r = Registry::<Timestamp>::with_max_age(w);
    r.add_transform(Transform::new("map","base",Vector3::zero(),q(),Stamp::At(Timestamp::from_nanos(1_000_000_000))).unwrap()).unwrap();
    r.add_transform(Transform::new("map","base",Vector3::zero(),q(),Stamp::At(Timestamp::from_nanos(2_000_000_000))).unwrap()).unwrap();
    println!("P8 'latest' via now(): {:?}", r.get_transform("map","base", Timestamp::now()).map(|t| t.translation().x).map_err(|e| e.to_string()));

    // P9 whirl_tf clear_dynamic pattern (buffer.rs:200-204): remove_frame per dynamic child; then a re-parented edge
    let mut r = Registry::<Timestamp>::with_max_age(w);
    r.add_transform(Transform::new("odom","base",Vector3::zero(),q(),Stamp::At(Timestamp::from_nanos(5))).unwrap()).unwrap();
    let _ = r.remove_frame("base");
    let re = r.add_transform(Transform::new("map","base",Vector3::zero(),q(),Stamp::At(Timestamp::from_nanos(1))).unwrap());
    let mut r2 = Registry::<Timestamp>::with_max_age(w);
    r2.add_transform(Transform::new("odom","base",Vector3::zero(),q(),Stamp::At(Timestamp::from_nanos(5))).unwrap()).unwrap();
    r2.remove_transforms_before(Timestamp::from_nanos(u64::MAX));
    let re2 = r2.add_transform(Transform::new("map","base",Vector3::zero(),q(),Stamp::At(Timestamp::from_nanos(1))).unwrap());
    println!("P9 after remove_frame, reparent insert ok={} ; after remove_transforms_before(MAX), reparent insert = {:?}", re.is_ok(), re2.map_err(|e| e.to_string()));
}
