use core::time::Duration;
use std::time::Instant;
use transforms::{Registry, geometry::{Quaternion, Transform, Vector3}, time::{Stamp, Timestamp}};

fn tf(i: usize, ns: u64) -> Transform<Timestamp> {
    let parent = if i == 0 { "root".to_string() } else { format!("f{}", i - 1) };
    Transform::new(&parent, &format!("f{i}"), Vector3::new(1.0, 0.0, 0.0), Quaternion::identity(), Stamp::At(Timestamp::from_nanos(ns))).unwrap()
}

fn main() {
    for (frames, samples) in [(10usize, 100u64), (30, 1000), (60, 2000)] {
        let mut out = Vec::new();
        for mode in ["loop", "batch"] {
            let mut r = Registry::with_max_age(Duration::from_nanos(samples - 1));
            for ns in 0..samples { for i in 0..frames { r.add_transform(tf(i, ns)).unwrap(); } }
            let iters = 2000u64;
            let msgs: Vec<Vec<_>> = (0..iters).map(|k| (0..frames).map(|i| tf(i, samples + k)).collect()).collect();
            let start = Instant::now();
            for m in msgs {
                if mode == "loop" { for t in m { r.add_transform(t).unwrap(); } } else { r.add_transforms(m).unwrap(); }
            }
            out.push(format!("{mode} {:.2} us", start.elapsed().as_secs_f64() * 1e6 / iters as f64));
        }
        println!("{frames} frames x {samples} samples: {}", out.join(", "));
    }
}
