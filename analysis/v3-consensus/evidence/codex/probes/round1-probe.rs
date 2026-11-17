use transforms::{Registry, geometry::{Transform, Quaternion, Vector3}, time::{Stamp, Timestamp}};
use std::time::Duration;
fn tf(p: &str, c: &str, x: f64, n: u64) -> Transform {
    Transform::new(p,c,Vector3::new(x,0.0,0.0),Quaternion::identity(),Stamp::At(Timestamp::from_nanos(n))).unwrap()
}
fn main() {
    let mut r = Registry::new();
    r.add_transforms([tf("a","b",1e308,0), tf("b","c",1e308,0)]).unwrap();
    println!("overflow_forward={:?}",r.get_transform("a","c",Timestamp::zero()).map(|x| x.translation()));
    println!("overflow_reverse={:?}",r.get_transform("c","a",Timestamp::zero()).map(|x| x.translation()));
    let q = Quaternion::from_wxyz(1.0+0.9e-6,0.0,0.0,0.0);
    let t: Transform = Transform::static_between("a","b",Vector3::zero(),q).unwrap();
    println!("accepted_norm={} rotated_1km={:?}",t.rotation().norm(),t.rotation().rotate_vector(Vector3::new(1000.0,0.0,0.0)));
    let (mut a,mut b,mut c) = (tf("a","b",1e100,0),tf("b","c",1e100,0),tf("c","a",1e100,0));
    for i in 1..2000 {
        let (aa,bb,cc) = if i%2==1 {((a.clone()*b.clone()).unwrap(),(b.clone()*c.clone()).unwrap(),(c.clone()*a.clone()).unwrap())} else {((a.clone()*c.clone()).unwrap(),(b.clone()*a.clone()).unwrap(),(c.clone()*b.clone()).unwrap())};
        (a,b,c)=(aa,bb,cc);
        if !a.translation().x.is_finite() {println!("bounded_seed_overflow_round={i} products={}",3*i); break;}
    }
    let mut r = Registry::with_max_age(Duration::from_nanos(10));
    r.add_transform(tf("a","b",100.0,100)).unwrap();
    println!("late_insert={:?}",r.add_transform(tf("a","b",50.0,50)));
    println!("late_lookup={:?}",r.get_transform("a","b",Timestamp::from_nanos(50)));
    let mut r = Registry::new();
    r.add_transforms([tf("map","odom",1.0,1),tf("map","odom",5.0,2),tf("odom","base",3.0,1),tf("odom","base",5.0,2)]).unwrap();
    let x=r.get_transform_at("odom",Timestamp::from_nanos(2),"base",Timestamp::from_nanos(1),"map").unwrap();
    println!("cross_time={:?} stamp={:?}",x.translation(),x.timestamp());
    r.add_transform(x).unwrap();
    println!("after_cross_time_insert={:?}",r.get_transform("odom","base",Timestamp::from_nanos(2)).unwrap().translation());
    let a=tf("a","b",0.0,1_700_000_000_000_000_000);let b=tf("b","c",0.0,1_700_000_000_000_000_001);
    let err=(a*b).unwrap_err();println!("mismatch_display={err} mismatch_debug={err:?}");
    let a=tf("a","",1.0,0);let b=tf("","b",2.0,0);
    println!("empty_name_composition={:?}",(a*b).map(|x| x.translation()));
}
