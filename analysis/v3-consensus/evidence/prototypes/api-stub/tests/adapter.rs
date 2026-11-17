#![allow(dead_code, clippy::pedantic)]
use core::time::Duration;
use std::borrow::Cow;
use transforms::*;

// 1. build from borrowed and owned
fn build(owned: String, borrowed: &str, sref: &String, cow: Cow<'_, str>, bx: Box<str>) -> Result<(), TransformError> {
    let v = Vector3::new(0.0, 0.0, 0.0);
    let r = Rotation::from_wxyz(1.0, 0.0, 0.0, 0.0)?;
    let _ = Transform::new(borrowed, owned, v, r, Stamp::Static)?;
    let _ = Transform::new(sref, cow, v, Rotation::IDENTITY, Stamp::At(Timestamp::from_nanos(0)))?;
    let _ = Transform::new(bx, 'c', v, r, Stamp::Static)?; // char: Into<String> compiles
    Ok(())
}
const V: Vector3 = Vector3::new(1.0, 2.0, 3.0);
const T: Timestamp = Timestamp::from_nanos(-5);
const N: i64 = T.as_nanos();
const A: Option<Timestamp> = Stamp::At(T).at();
const R: Rotation = Rotation::IDENTITY;

// 2. insert + batch with index
fn batch(reg: &mut Registry, msgs: &[Transform]) -> Result<(), String> {
    reg.insert(&msgs[0]).map_err(|e| e.to_string())?;
    match reg.insert_all(msgs) {
        Ok(()) => Ok(()),
        Err(BatchError { index, error }) => {
            let bad = &msgs[index];
            Err(format!("member {index} ({} <- {}): {error}", bad.target(), bad.source()))
        }
    }
}
fn batch_source(e: &BatchError) -> bool {
    std::error::Error::source(e).and_then(|s| s.downcast_ref::<InsertError>()).is_some()
}

// 3. match every LookupError variant
enum Wait { Retry, Fail(String) }
fn classify(e: LookupError) -> Wait {
    match e {
        LookupError::UnknownFrame { frame, .. } => { let _ = frame; Wait::Retry }
        LookupError::Disconnected { a, b, .. } => { let _ = (a, b); Wait::Retry }
        LookupError::NoData { .. } => Wait::Retry,
        LookupError::OutOfRange { requested, earliest, latest, frame, .. } => {
            if requested > latest { Wait::Retry } else { let _ = earliest; Wait::Fail(frame) }
        }
        LookupError::NoCommonTime { frame, .. } => Wait::Fail(frame),
        LookupError::NonFinite => Wait::Fail("non-finite".into()),
        other => Wait::Fail(other.to_string()),
    }
}
// 4. wait loop
fn wait(reg: &Registry, mut pump: impl FnMut() -> bool) -> Result<Transform, String> {
    let at = Timestamp::from_nanos(10);
    loop {
        match reg.lookup("map", "lidar", at) {
            Ok(t) => return Ok(t),
            Err(e) => match classify(e) { Wait::Retry => if !pump() { return Err("timeout".into()) }, Wait::Fail(s) => return Err(s) },
        }
    }
}
// 5. flatten and republish
fn flatten(reg: &mut Registry, at: Timestamp) -> Result<(), Box<dyn std::error::Error>> {
    let t = reg.lookup("map", "lidar", at)?;
    reg.insert(&t)?; // rejected as ParentMismatch unless lidar's parent is map
    let renamed = Transform::new("map", "lidar_flat", t.translation(), t.rotation(), t.stamp())?;
    reg.insert(&renamed)?;
    let s = Transform::new(t.target(), "lidar_static", t.translation(), t.rotation(), Stamp::Static)?;
    reg.insert(&s)?;
    let inv = t.inverse()?;
    let _id = t.compose(&inv)?;
    Ok(())
}
// 6. cross-time
fn cross(reg: &Registry) -> Result<Vector3, LookupError> {
    let c = reg.lookup_across("base", Timestamp::from_nanos(20), "base", Timestamp::from_nanos(10), "odom")?;
    let p = c.rotation().rotate(Vector3::new(1.0, 0.0, 0.0));
    let t = c.translation();
    let _ = (c.target(), c.source(), c.target_time(), c.source_time());
    Ok(Vector3::new(p.x + t.x, p.y + t.y, p.z + t.z))
}
// 7. latest then lookup
fn latest(reg: &Registry) -> Result<(Stamp, Transform), LookupError> {
    let stamp = reg.latest_common_time("map", "lidar")?;
    let at = match stamp { Stamp::At(t) => t, Stamp::Static => Timestamp::from_nanos(0) /* arbitrary: nothing says which */ };
    Ok((stamp, reg.lookup("map", "lidar", at)?))
}
// 8. expiry recovery keeping the sample
fn recover(reg: &mut Registry, tf: &Transform) -> Result<(), InsertError> {
    match reg.insert(tf) {
        Err(InsertError::Expired { frame, stamp, newest, .. }) if stamp < newest => {
            reg.remove_frame(&frame); // frame == tf.source(), presumably
            reg.insert(tf)
        }
        r => r,
    }
}
// 9. ROS stamp
fn ros_in(sec: i32, nanosec: u32) -> Option<Timestamp> { Timestamp::from_secs_nanos(i64::from(sec), nanosec) }
fn ros_out(t: Timestamp) -> Option<(i32, u32)> { let (s, n) = t.as_secs_nanos(); Some((i32::try_from(s).ok()?, n)) }
fn age(a: Timestamp, b: Timestamp) -> Option<Duration> { a.checked_duration_since(b) }
fn shift(a: Timestamp) -> Option<Timestamp> { a.checked_sub(Duration::from_secs(1))?.checked_add(Duration::MAX) }

// 10. serde types
#[cfg(feature = "serde")]
fn roundtrip(t: &Transform) {
    let s = serde_json::to_string(t).unwrap();
    let back: Transform = serde_json::from_str(&s).unwrap();
    assert_eq!(&back, t);
    let b = postcard::to_allocvec(&(Vector3::new(0.0,0.0,0.0), Rotation::IDENTITY, Timestamp::MAX, Stamp::Static)).unwrap();
    let _: (Vector3, Rotation, Timestamp, Stamp) = postcard::from_bytes(&b).unwrap();
}
fn bounds() {
    fn ss<X: Send + Sync + 'static>() {}
    fn err<X: core::error::Error + Clone + PartialEq + Send + Sync + 'static>() {}
    ss::<Registry>(); ss::<Transform>(); ss::<CrossTimeTransform>();
    err::<TransformError>(); err::<InsertError>(); err::<LookupError>(); err::<BatchError>();
    fn key<X: Ord + core::hash::Hash + core::fmt::Display + Copy>() {}
    key::<Timestamp>();
}
#[test] fn compiles() {}
