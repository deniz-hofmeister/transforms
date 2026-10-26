//! add_transforms must equal the add_transform loop on success, and leave
//! the registry untouched with the loop's first error on failure.
use core::time::Duration;
use proptest::prelude::*;
use transforms::{
    Registry,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

const FRAMES: [&str; 5] = ["a", "b", "c", "d", "e"];

#[derive(Debug, Clone)]
struct Op { parent: usize, child: usize, stat: bool, ns: u64, x: f64, bad: bool }

fn op() -> impl Strategy<Value = Op> {
    (0..5usize, 0..5usize, prop::bool::weighted(0.25), 0..8u64, -10.0..10.0f64, prop::bool::weighted(0.05))
        .prop_map(|(parent, child, stat, ns, x, bad)| Op { parent, child, stat, ns, x, bad })
}

fn build(o: &Op) -> Transform<Timestamp> {
    let t = Transform::new(
        FRAMES[o.parent], FRAMES[o.child], Vector3::new(o.x, 0.0, 0.0),
        Quaternion::identity(),
        if o.stat { Stamp::Static } else { Stamp::At(Timestamp::from_nanos(o.ns)) },
    );
    match t {
        Ok(t) => t,
        // self-referential or similar: fall back to a NaN-free but still raw case
        Err(e) => panic!("{e:?}"),
    }
}

fn snapshot(r: &Registry) -> Vec<String> {
    let mut out = Vec::new();
    for a in FRAMES { for b in FRAMES {
        out.push(format!("{:?}", r.latest_common_time(a, b)));
        for ns in 0..9 { out.push(format!("{:?}", r.get_transform(a, b, Timestamp::from_nanos(ns)))); }
    }}
    out
}

fn fresh(max_age: Option<u64>) -> Registry {
    match max_age { Some(n) => Registry::with_max_age(Duration::from_nanos(n)), None => Registry::new() }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20_000))]
    #[test]
    fn batch_equals_loop(max_age in prop::option::of(0..4u64), seed in prop::collection::vec(op(), 0..10), batch in prop::collection::vec(op(), 0..7)) {
        // Transform::new rejects self-reference? keep only buildable ops.
        let ok = |_: &Op| true;
        let seed: Vec<_> = seed.into_iter().filter(|o| ok(o) && !o.bad).collect();
        let batch: Vec<_> = batch.into_iter().filter(ok).collect();
        let make = |o: &Op| if o.bad {
            // A derived transform whose translation overflowed: not
            // re-validated by composition, rejected by insertion.
            let stamp = if o.stat { Stamp::Static } else { Stamp::At(Timestamp::from_nanos(o.ns)) };
            let a = Transform::new(FRAMES[o.parent], "via", Vector3::new(1.7e308, 0.0, 0.0), Quaternion::identity(), stamp).unwrap();
            let b = Transform::new("via", FRAMES[o.child], Vector3::new(1.7e308, 0.0, 0.0), Quaternion::identity(), stamp).unwrap();
            (a * b).unwrap()
        } else { build(o) };
        let mut seq = fresh(max_age);
        let mut bat = fresh(max_age);
        for o in &seed { let _ = seq.add_transform(build(o)); let _ = bat.add_transform(build(o)); }
        let before = snapshot(&bat);
        let mut expected = Ok(());
        for o in &batch { if let Err(e) = seq.add_transform(make(o)) { expected = Err(e); break; } }
        let got = bat.add_transforms(batch.iter().map(make));
        prop_assert_eq!(format!("{got:?}"), format!("{expected:?}"));
        if got.is_ok() { prop_assert_eq!(snapshot(&bat), snapshot(&seq)); }
        else { prop_assert_eq!(snapshot(&bat), before); }
    }
}
