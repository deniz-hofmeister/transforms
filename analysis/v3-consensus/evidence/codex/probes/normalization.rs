use transforms::geometry::Quaternion;
fn norm2(q: Quaternion)-> f64 { q.w*q.w+q.x*q.x+q.y*q.y+q.z*q.z }
fn canonical(q: Quaternion)->Quaternion {
    let q=if (norm2(q)-1.0).abs() > 8.0*f64::EPSILON {q.normalize().unwrap()} else {q};
    assert!((norm2(q)-1.0).abs()<=8.0*f64::EPSILON);
    q
}
fn main() {
    let step=Quaternion::from_wxyz(0.9999998750000026,0.00028867512256668215,0.00028867512256668215,0.00028867512256668215).normalize().unwrap();
    let mut raw=Quaternion::identity();
    let mut guarded=raw;
    let mut max_error=0.0f64;
    for _ in 0..1_000_000 {
        raw=raw*step;
        guarded=canonical(guarded*step);
        max_error=max_error.max((norm2(guarded)-1.0).abs());
        assert_eq!(canonical(guarded),guarded);
    }
    println!("one_million_products raw_norm2_error={:.17e} guarded_max_norm2_error={max_error:.17e}",norm2(raw)-1.0);
    let mut state=17u64;
    for _ in 0..100_000 {
        let mut component=|| {state=state.wrapping_mul(6364136223846793005).wrapping_add(1); ((state>>11) as f64)/(1u64<<53) as f64-0.5};
        let q=Quaternion::from_wxyz(component(),component(),component(),component()).normalize().unwrap();
        let q=canonical(q);
        guarded=canonical(guarded*q);
        assert_eq!(canonical(guarded),guarded);
    }
    println!("random_products=100000 fixed_point_checks=passed");
}
