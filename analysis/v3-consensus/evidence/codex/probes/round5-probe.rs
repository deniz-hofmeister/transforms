
use transforms::{Transform, geometry::{Quaternion as Q,Vector3 as V}, time::{Stamp as OldStamp,Timestamp as OldTime}};
use serde::{Deserialize,Serialize};
#[derive(Debug,Clone,Serialize,Deserialize)]
enum Stamp { Static, At(i64) }
// Candidate M10 order: translation, rotation, target, source, stamp.
#[derive(Debug,Clone,Serialize,Deserialize)]
struct Wire { translation: V, rotation: Q, target: String, source: String, stamp: Stamp }
#[derive(Debug,Deserialize)]
#[serde(try_from="Wire")]
struct NewTransform(Wire);
fn n2(q: Q)->f64 { q.w*q.w+q.x*q.x+q.y*q.y+q.z*q.z }
fn fix(q:Q)->Q { if (n2(q)-1.0).abs()<=8.0*f64::EPSILON {q} else {q.normalize().unwrap()} }
impl TryFrom<Wire> for NewTransform {
    type Error=&'static str;
    fn try_from(mut w:Wire)->Result<Self,Self::Error> {
        if ![w.translation.x,w.translation.y,w.translation.z,w.rotation.w,w.rotation.x,w.rotation.y,w.rotation.z].iter().all(|x|x.is_finite()) {return Err("nonfinite")}
        if (w.rotation.norm()-1.0).abs()>1e-6 {return Err("nonunit")}
        w.rotation=fix(w.rotation);
        if (n2(w.rotation)-1.0).abs()>8.0*f64::EPSILON {return Err("postcondition")}
        if w.target.is_empty() || w.source.is_empty() {return Err("empty frame")}
        if w.target==w.source && (w.translation!=V::zero() || w.rotation!=Q::identity()) {return Err("self not identity")}
        Ok(Self(w))
    }
}
fn compose(a:&Wire,b:&Wire)->Wire {
    assert_eq!(a.source,b.target);
    Wire{target:a.target.clone(),source:b.source.clone(),stamp:a.stamp.clone(),translation:a.rotation.rotate_vector(b.translation)+a.translation,rotation:fix(a.rotation*b.rotation)}
}
fn inverse(a:&Wire)->Wire {
    let q=a.rotation.conjugate();let r=q.rotate_vector(a.translation);let s=-1.0/n2(a.rotation);
    Wire{target:a.source.clone(),source:a.target.clone(),stamp:a.stamp.clone(),rotation:q,translation:V::new(s*r.x,s*r.y,s*r.z)}
}
fn wire(a:&str,b:&str,t:V,q:Q)->Wire {Wire{target:a.into(),source:b.into(),translation:t,rotation:q,stamp:Stamp::At(0)}}
fn from_ros(sec:i32,nanosec:u32)->Option<i64>{let nanosec=i64::from(nanosec);if nanosec>=1_000_000_000{return None} Some(i64::from(sec)*1_000_000_000+nanosec)}
fn to_ros(nanos:i64)->Option<(i32,u32)>{Some((i32::try_from(nanos.div_euclid(1_000_000_000)).ok()?,u32::try_from(nanos.rem_euclid(1_000_000_000)).ok()?))}
fn main(){
println!("fixture transform_postcard_bytes_are_frozen: {:?}", postcard::from_bytes::<NewTransform>(&[0, 0, 0, 0, 0, 0, 248, 63, 0, 0, 0, 0, 0, 0, 2, 192, 0, 0, 0, 0, 0, 0, 9, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 128, 128, 180, 197, 150, 183, 154, 170, 24, 3, 109, 97, 112, 9, 98, 97, 115, 101, 95, 108, 105, 110, 107]));
println!("fixture static_transform_postcard_bytes_are_frozen: {:?}", postcard::from_bytes::<NewTransform>(&[0, 0, 0, 0, 0, 0, 248, 63, 0, 0, 0, 0, 0, 0, 2, 192, 0, 0, 0, 0, 0, 0, 9, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 109, 97, 112, 9, 98, 97, 115, 101, 95, 108, 105, 110, 107]));
println!("fixture transform_with_a_general_rotation_postcard_bytes_are_frozen: {:?}", postcard::from_bytes::<NewTransform>(&[0, 0, 0, 0, 0, 0, 248, 63, 0, 0, 0, 0, 0, 0, 2, 192, 0, 0, 0, 0, 0, 0, 9, 64, 23, 93, 116, 209, 69, 23, 237, 63, 70, 23, 93, 116, 209, 69, 183, 63, 70, 23, 93, 116, 209, 69, 199, 63, 70, 23, 93, 116, 209, 69, 215, 63, 1, 128, 128, 180, 197, 150, 183, 154, 170, 24, 3, 109, 97, 112, 9, 98, 97, 115, 101, 95, 108, 105, 110, 107]));
println!("fixture point_postcard_bytes_are_frozen: {:?}", postcard::from_bytes::<NewTransform>(&[0, 0, 0, 0, 0, 0, 248, 63, 0, 0, 0, 0, 0, 0, 2, 192, 0, 0, 0, 0, 0, 0, 9, 64, 0, 0, 0, 0, 0, 0, 240, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 128, 180, 197, 150, 183, 154, 170, 24, 6, 99, 97, 109, 101, 114, 97]));

let old:Transform=Transform::new("B","C",V::new(1.0,2.0,3.0),Q::identity(),OldStamp::At(OldTime::from_nanos(65))).unwrap();
let bytes=postcard::to_allocvec(&old).unwrap();
let new=postcard::from_bytes::<NewTransform>(&bytes).unwrap();
println!("OLD VALID B<-C @65ns -> NEW VALID: {:?}",new.0);
let good=wire("map","base",V::new(1.0,2.0,3.0),Q::identity());
println!("v3 own wire roundtrip: {:?}",postcard::from_bytes::<NewTransform>(&postcard::to_allocvec(&good).unwrap()));
let a=wire("a","b",V::new(1.0,0.0,0.0),Q::identity());
let b=wire("b","a",V::new(1.0,0.0,0.0),Q::identity());
assert!(NewTransform::try_from(a.clone()).is_ok());assert!(NewTransform::try_from(b.clone()).is_ok());
let c=compose(&a,&b);
println!("opposite labels compose: {:?}; reconstruction={:?}; inverse={:?}",c,NewTransform::try_from(c.clone()),inverse(&c));
let a=wire("a","b",V::new(1.1,-3.3,4.7),Q::from_wxyz(10.0/11.0,1.0/11.0,2.0/11.0,4.0/11.0));
let c=compose(&a,&inverse(&a));
println!("a compose inverse(a): {:?}; decode={:?}",c,postcard::from_bytes::<NewTransform>(&postcard::to_allocvec(&c).unwrap()));
for sec in [i32::MIN,-1,0,1,i32::MAX] {for ns in [0,1,500_000_000,999_999_999]{assert_eq!(to_ros(from_ros(sec,ns).unwrap()),Some((sec,ns)));}}
assert_eq!(from_ros(0,1_000_000_000),None);assert_eq!(from_ros(0,u32::MAX),None);
assert_eq!(to_ros(-1),Some((-1,999_999_999)));assert_eq!(to_ros(i64::MIN),None);assert_eq!(to_ros(i64::MAX),None);
println!("ROS conversion boundary checks passed");

let mut registry = transforms::Registry::new();
for (t,x) in [(0,0.0),(10,2.0)] {
 registry.add_transform(Transform::new("world","camera",V::new(x,0.0,0.0),Q::identity(),OldStamp::At(OldTime::from_nanos(t))).unwrap()).unwrap();
}
println!("cross-time same-frame motion baseline: {:?}",registry.get_transform_at("camera",OldTime::from_nanos(10),"camera",OldTime::from_nanos(0),"world"));
registry.add_transform(Transform::static_between("other","x",V::zero(),Q::identity()).unwrap()).unwrap();
println!("same-time same-frame but disconnected fixed baseline: {:?}",registry.get_transform_at("camera",OldTime::from_nanos(0),"camera",OldTime::from_nanos(0),"other"));
let early=1_000_000_000_i64;let later=2_000_000_000_i64;
println!("future stamp naive Duration age: {:?}",core::time::Duration::from_nanos((early-later) as u64));
println!("future stamp checked_sub alone: {:?}",early.checked_sub(later));
}
