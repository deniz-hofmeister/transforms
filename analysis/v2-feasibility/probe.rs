//! Standalone, safe-Rust differential and heap-measurement client.
//! Compile against each revision with the accompanying run.py script.

use std::{hint::black_box, time::Duration};
use transforms::{
    Registry,
    errors::RegistryError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

fn sample(
    parent: &str,
    child: &str,
    nanos: u64,
    variant: u64,
    static_edge: bool,
) -> Transform {
    let rotations = [
        Quaternion::identity(),
        Quaternion::from_wxyz(0.5, 0.5, -0.5, 0.5),
        Quaternion::from_wxyz(1.0 - 9e-7, 0.0, 0.0, 0.0),
        Quaternion::from_wxyz(-0.5, 0.5, -0.5, 0.5),
    ];
    let translation = match variant % 5 {
        0 => Vector3::new(-0.0, 2.0, -3.0),
        1 => Vector3::new(1e308, -1e308, 1e308),
        _ => Vector3::new(3.125, -0.125, 9.75),
    };
    Transform::new(
        parent,
        child,
        translation,
        rotations[(variant % 4) as usize],
        if static_edge {
            Stamp::Static
        } else {
            Stamp::At(Timestamp::from_nanos(nanos))
        },
    )
    .unwrap()
}

fn print_result(result: Result<Transform, RegistryError>) {
    match result {
        Ok(t) => {
            let p = t.translation();
            let q = t.rotation();
            let bits = [p.x, p.y, p.z, q.w, q.x, q.y, q.z].map(f64::to_bits);
            println!(
                "ok {} {} {:?} {bits:x?}",
                t.parent(),
                t.child(),
                t.timestamp()
            );
        }
        Err(e) => println!("err {e:?} | {e}"),
    }
}

fn trace() {
    let names = ["root", "a", "b", "c", "d", "other", "e", "missing"];
    let mut random = 0x2a_39_71_85_u64;
    for retention in [None, Some(Duration::ZERO), Some(Duration::from_nanos(12))] {
        let mut registry = retention.map_or_else(Registry::new, Registry::with_max_age);
        for step in 0..800 {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            let child = 1 + (random >> 16) as usize % 6;
            let parent = (random >> 32) as usize % child;
            let nanos = (random >> 8) % 32;
            match random % 10 {
                0 => println!(
                    "remove {} {}",
                    names[child],
                    registry.remove_frame(names[child])
                ),
                1 => {
                    registry.remove_transforms_before(Timestamp::from_nanos(nanos));
                    println!("clear {nanos}");
                }
                _ => println!(
                    "insert {:?}",
                    registry.add_transform(sample(
                        names[parent],
                        names[child],
                        nanos,
                        random >> 40,
                        random % 7 == 0
                    ))
                ),
            }
            if step % 5 == 0 {
                for target in names {
                    for source in names {
                        println!("case {retention:?} {step} {target} {source}");
                        println!("latest {:?}", registry.latest_common_time(target, source));
                        for time in [0, 8, 16, 24, 31, 32] {
                            print_result(registry.get_transform(
                                target,
                                source,
                                Timestamp::from_nanos(time),
                            ));
                        }
                        print_result(registry.get_transform_at(
                            target,
                            Timestamp::from_nanos(24),
                            source,
                            Timestamp::from_nanos(8),
                            "root",
                        ));
                    }
                }
            }
        }
    }
}

fn populated(
    samples: u64,
    name_length: usize,
    depth: usize,
) -> (Registry, Vec<String>) {
    let names: Vec<_> = (0..=depth).map(|i| format!("{i:0name_length$}")).collect();
    let mut registry = Registry::new();
    for edge in names.windows(2) {
        for time in 0..samples {
            registry
                .add_transform(sample(&edge[0], &edge[1], time * 2, 3, false))
                .unwrap();
        }
    }
    (registry, names)
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args[1].as_str() {
        "trace" => trace(),
        "memory" => {
            let (registry, names) =
                populated(args[2].parse().unwrap(), args[3].parse().unwrap(), 1);
            black_box((&registry, &names));
        }
        "lookup" | "time_lookup" => {
            let iterations: usize = args[2].parse().unwrap();
            let depth: usize = args[3].parse().unwrap();
            let (registry, names) = populated(1000, 8, depth);
            let (target, source) = if args[4] == "reverse" {
                (&names[depth], &names[0])
            } else {
                (&names[0], &names[depth])
            };
            let time = if args[5] == "exact" { 1000 } else { 1001 };
            let started = std::time::Instant::now();
            for _ in 0..iterations {
                black_box(
                    registry
                        .get_transform(
                            black_box(target),
                            black_box(source),
                            Timestamp::from_nanos(time),
                        )
                        .unwrap(),
                );
            }
            if args[1] == "time_lookup" {
                println!("{}", started.elapsed().as_nanos());
            }
        }
        "insert" => {
            let size: u64 = args[2].parse().unwrap();
            let iterations: u64 = args[3].parse().unwrap();
            let (mut registry, names) = populated(size, 8, 1);
            let start = std::time::Instant::now();
            for i in 0..iterations {
                let at = if args[4] == "ordered" {
                    (size + i) * 2
                } else {
                    (i * 7919 % size) * 2 + 1
                };
                registry
                    .add_transform(black_box(sample(&names[0], &names[1], at, 3, false)))
                    .unwrap();
            }
            println!("{}", start.elapsed().as_nanos());
            black_box(registry);
        }
        other => panic!("unknown probe {other}"),
    }
}
