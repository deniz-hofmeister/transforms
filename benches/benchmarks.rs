use core::time::Duration;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{cell::Cell, hint::black_box};
use transforms::{Registry, Rotation, Stamp, Timestamp, Transform, Vector3};

/// Base timestamp for dynamic samples.
const BASE_NANOS: i64 = 1_000_000_000;
/// Nanoseconds between consecutive samples in the prepared registries.
const SAMPLE_INTERVAL_NANOS: i64 = 1_000_000;
/// A query instant halfway between two samples in the middle of 1k samples,
/// so every dynamic hop interpolates.
const MID_QUERY_NANOS: i64 = BASE_NANOS + 500 * SAMPLE_INTERVAL_NANOS + SAMPLE_INTERVAL_NANOS / 2;
/// How fast the dynamic samples below rotate. Consecutive samples must
/// differ in rotation, or every interpolated lookup takes slerp's
/// normalized-lerp shortcut and the sin-weighted branch, the dominant
/// float cost on a soft-float target, is never measured at all.
const RADIANS_PER_SECOND: f64 = 1.0;
/// The normalized (1, 1, 1) axis: aligned with no coordinate axis, so a
/// rotation about it exercises every term of the quaternion product
/// instead of leaving two thirds of them zero.
const AXIS: f64 = 0.577_350_269_189_625_8;

/// The rotation a sample stamped `nanos` carries.
fn rotation_at(nanos: i64) -> Rotation {
    let half_angle = (nanos - BASE_NANOS) as f64 * 1e-9 * RADIANS_PER_SECOND / 2.0;
    let (sin, cos) = (half_angle.sin(), half_angle.cos());
    Rotation::from_wxyz(cos, sin * AXIS, sin * AXIS, sin * AXIS).unwrap()
}

fn transform_at(
    parent: &str,
    child: &str,
    nanos: i64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(1.0, 0.0, 0.0),
        rotation_at(nanos),
        Stamp::At(Timestamp::from_nanos(nanos)),
    )
    .unwrap()
}

/// A static edge between two frames with the given offset and no rotation.
fn static_mount(
    parent: &str,
    child: &str,
    translation: Vector3,
) -> Transform {
    Transform::new(
        parent,
        child,
        translation,
        Rotation::IDENTITY,
        Stamp::Static,
    )
    .unwrap()
}

/// A static edge between two frames, with no offset or rotation.
fn static_edge(
    parent: &str,
    child: &str,
) -> Transform {
    static_mount(parent, child, Vector3::new(0.0, 0.0, 0.0))
}

/// Inserts `samples` dynamic transforms on every edge of `edges`, spaced
/// `SAMPLE_INTERVAL_NANOS` apart starting at `BASE_NANOS`. Returns the
/// first free timestamp after the samples.
fn fill(
    registry: &mut Registry,
    edges: &[(&str, &str)],
    samples: i64,
) -> i64 {
    let mut nanos = BASE_NANOS;
    for _ in 0..samples {
        for (parent, child) in edges {
            registry
                .insert(&transform_at(parent, child, nanos))
                .unwrap();
        }
        nanos += SAMPLE_INTERVAL_NANOS;
    }
    nanos
}

/// A registry pre-warmed with `samples` dynamic transforms between "a" and
/// "b". Returns the registry and the first free timestamp after the samples.
fn prewarmed_registry(samples: i64) -> (Registry, i64) {
    let mut registry = Registry::unbounded();
    let next = fill(&mut registry, &[("a", "b")], samples);
    (registry, next)
}

/// Steady-state insert: the registry is pre-warmed and bounded by `max_age`,
/// each iteration inserts the next sample of the stream. The transform is
/// built in the batch setup and dropped outside the timed routine, so only
/// `insert` is measured.
fn insert_prewarmed(
    b: &mut criterion::Bencher,
    max_age: Duration,
    samples: i64,
) {
    let mut registry = Registry::with_max_age(max_age);
    let next = Cell::new(fill(&mut registry, &[("a", "b")], samples));
    b.iter_batched_ref(
        || {
            let nanos = next.get();
            next.set(nanos + SAMPLE_INTERVAL_NANOS);
            transform_at("a", "b", nanos)
        },
        |transform| registry.insert(black_box(transform)).unwrap(),
        BatchSize::SmallInput,
    );
}

fn benchmark_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("insert_prewarmed_1k", |b| {
        insert_prewarmed(b, Duration::from_secs(1), 1000);
    });

    // 60 s max_age at 1 kHz sample spacing keeps ~60k live entries, the
    // README Quick Start configuration. Guards the eviction path: a return
    // to full-buffer scanning on insert shows up here as a ~100x regression.
    group.bench_function("insert_prewarmed_60k", |b| {
        insert_prewarmed(b, Duration::from_secs(60), 60_000);
    });

    // 100k resident samples: pins the eviction fix at depth. The old
    // full-map retain scan made this insert O(resident samples), ~1800x
    // slower than the empty-registry case.
    group.bench_function("insert_prewarmed_100k", |b| {
        insert_prewarmed(b, Duration::from_secs(100), 100_000);
    });

    group.finish();
}

/// One message carrying the next sample of eight frames, stored as a batch:
/// the preflight with its running newest stamps, then the commit. Each
/// frame holds 1k samples under a 1 s window, so every member also evicts.
fn benchmark_insert_all(c: &mut Criterion) {
    const EDGES: [(&str, &str); 8] = [
        ("map", "odom"),
        ("odom", "base"),
        ("base", "arm_1"),
        ("arm_1", "arm_2"),
        ("arm_2", "arm_3"),
        ("arm_3", "tool"),
        ("base", "wheel_left"),
        ("base", "wheel_right"),
    ];
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("insert_all_8_frames_prewarmed_1k", |b| {
        let mut registry = Registry::with_max_age(Duration::from_secs(1));
        let next = Cell::new(fill(&mut registry, &EDGES, 1000));
        b.iter_batched_ref(
            || {
                let nanos = next.get();
                next.set(nanos + SAMPLE_INTERVAL_NANOS);
                EDGES.map(|(parent, child)| transform_at(parent, child, nanos))
            },
            |batch| registry.insert_all(black_box(batch.as_slice())).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Moving a dynamic frame between two parents: the checks, the cycle walk
/// from the new parent, and the swap of the frame's history for the seed.
/// The frame hangs four edges deep and has a child of its own.
fn benchmark_reparent(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("reparent", |b| {
        let mut registry = Registry::unbounded();
        for (parent, child) in [("map", "odom"), ("odom", "base"), ("base", "arm")] {
            registry.insert(&static_edge(parent, child)).unwrap();
        }
        registry.insert(&static_edge("base", "tray")).unwrap();
        registry
            .insert(&transform_at("arm", "object", BASE_NANOS))
            .unwrap();
        registry.insert(&static_edge("object", "label")).unwrap();

        // `object` starts under `arm`, so the first move is to `tray`.
        let seeds = [
            transform_at("tray", "object", BASE_NANOS),
            transform_at("arm", "object", BASE_NANOS),
        ];
        let turn = Cell::new(0);
        b.iter(|| {
            let seed = &seeds[turn.get()];
            turn.set(1 - turn.get());
            registry.reparent(black_box(seed)).unwrap();
        });
    });

    group.finish();
}

/// Lookup at an exactly stored timestamp in a buffer of 1000 samples.
fn benchmark_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("lookup_1k", |b| {
        let (registry, next) = prewarmed_registry(1000);
        let query = Timestamp::from_nanos(next - SAMPLE_INTERVAL_NANOS);

        b.iter(|| black_box(registry.lookup("a", "b", query)).unwrap());
    });

    group.finish();
}

/// Lookup between two stored samples, forcing interpolation.
fn benchmark_lookup_interpolated(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("lookup_interpolated_1k", |b| {
        let (registry, _) = prewarmed_registry(1000);
        let query = Timestamp::from_nanos(MID_QUERY_NANOS);

        b.iter(|| black_box(registry.lookup("a", "b", query)).unwrap());
    });

    group.finish();
}

/// A realistic small robot tree mixing static mounts and dynamic edges:
/// map -> odom -> base_link -> {laser, imu, camera -> camera_optical}.
/// The three dynamic edges hold `samples` samples each; the three mounts
/// are static. Deep synthetic chains overstate real-world lookup costs;
/// this is the shape most robots actually query.
fn robot_tree(samples: i64) -> Registry {
    let mut registry = Registry::unbounded();

    for mount in [
        static_mount("base_link", "laser", Vector3::new(0.2, 0.0, 0.1)),
        static_mount("base_link", "imu", Vector3::new(0.0, 0.0, 0.05)),
        static_edge("camera", "camera_optical"),
    ] {
        registry.insert(&mount).unwrap();
    }
    fill(
        &mut registry,
        &[
            ("map", "odom"),
            ("odom", "base_link"),
            ("base_link", "camera"),
        ],
        samples,
    );
    registry
}

/// Interpolating lookups on the realistic robot tree: a full chain from the
/// map into a sensor's optical frame, a leaf-to-leaf query across the
/// common parent, and a cross-time query between two leaves through the
/// map.
fn benchmark_robot_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    let registry = robot_tree(1000);
    let query = Timestamp::from_nanos(MID_QUERY_NANOS);
    let earlier = Timestamp::from_nanos(MID_QUERY_NANOS - 100 * SAMPLE_INTERVAL_NANOS);

    group.bench_function("robot_tree_map_to_camera_optical", |b| {
        b.iter(|| black_box(registry.lookup("map", "camera_optical", query)).unwrap());
    });
    group.bench_function("robot_tree_leaf_to_leaf", |b| {
        b.iter(|| black_box(registry.lookup("laser", "camera_optical", query)).unwrap());
    });
    group.bench_function("robot_tree_lookup_across", |b| {
        b.iter(|| {
            black_box(registry.lookup_across("laser", query, "camera_optical", earlier, "map"))
                .unwrap()
        });
    });

    group.finish();
}

/// A 3-hop chain of dynamic edges, each holding 1k samples, queried between
/// samples so every hop interpolates.
fn benchmark_dynamic_chain_interpolated(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("dynamic_chain_3hop_interpolated_1k", |b| {
        let mut registry = Registry::unbounded();
        fill(&mut registry, &[("a", "b"), ("b", "c"), ("c", "d")], 1000);
        let query = Timestamp::from_nanos(MID_QUERY_NANOS);

        b.iter(|| black_box(registry.lookup("a", "d", query)).unwrap());
    });

    group.finish();
}

/// A 4-hop dynamic chain a -> b -> c -> d -> e with 1k samples per edge.
fn four_hop_chain() -> Registry {
    let mut registry = Registry::unbounded();
    fill(
        &mut registry,
        &[("a", "b"), ("b", "c"), ("c", "d"), ("d", "e")],
        1000,
    );
    registry
}

/// A 4-hop dynamic chain, queried in both directions between samples so
/// every hop interpolates.
///
/// A lookup toward an ancestor, `lookup("a", "e", t)`, the documented
/// direction, resolves entirely from the source half and inverts nothing;
/// the reverse inverts once. Benchmarking one direction only is how a
/// per-hop inversion stayed unmeasured through several releases while
/// costing the documented direction 2x.
fn benchmark_chain_both_directions(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    let registry = four_hop_chain();
    let query = Timestamp::from_nanos(MID_QUERY_NANOS);

    group.bench_function("dynamic_chain_4hop_toward_ancestor", |b| {
        b.iter(|| black_box(registry.lookup("a", "e", query)).unwrap());
    });
    group.bench_function("dynamic_chain_4hop_toward_descendant", |b| {
        b.iter(|| black_box(registry.lookup("e", "a", query)).unwrap());
    });

    group.finish();
}

/// Two leaves below a shared stack of 16 dynamic ancestors, each edge
/// holding 1k samples: r -> n1 -> ... -> n16 -> {left, right}. The
/// connecting chain is the two leaf edges; a resolver that samples each
/// leaf's whole ancestry and then discards the shared part pays for 34
/// interpolations where 2 are needed.
fn benchmark_siblings_below_dynamic_ancestors(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("siblings_below_16_dynamic_ancestors", |b| {
        let mut names = vec!["r".to_string()];
        names.extend((1..=16).map(|i| format!("n{i}")));
        let mut edges: Vec<(&str, &str)> = names
            .windows(2)
            .map(|pair| (pair[0].as_str(), pair[1].as_str()))
            .collect();
        edges.push(("n16", "left"));
        edges.push(("n16", "right"));

        let mut registry = Registry::unbounded();
        fill(&mut registry, &edges, 1000);
        let query = Timestamp::from_nanos(MID_QUERY_NANOS);

        b.iter(|| black_box(registry.lookup("left", "right", query)).unwrap());
    });

    group.finish();
}

/// The coverage query: two ancestry walks and one range read per hop, no
/// composition. As the first call of the two-call freshest-pose idiom its
/// cost rides on every reader tick, so it is measured alongside the
/// lookups it precedes: on the 4-hop chain, and on a 64-hop dynamic chain
/// where the per-hop cost dominates.
fn benchmark_latest_common_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    let registry = four_hop_chain();
    group.bench_function("latest_common_time_4hop", |b| {
        b.iter(|| black_box(registry.latest_common_time("a", "e")).unwrap());
    });

    group.bench_function("latest_common_time_64hop", |b| {
        let names: Vec<String> = (0..=64).map(|i| i.to_string()).collect();
        let edges: Vec<(&str, &str)> = names
            .windows(2)
            .map(|pair| (pair[0].as_str(), pair[1].as_str()))
            .collect();
        let mut registry = Registry::unbounded();
        fill(&mut registry, &edges, 100);

        b.iter(|| black_box(registry.latest_common_time("0", "64")).unwrap());
    });

    group.finish();
}

/// The failure path: a query one sample interval past the newest sample,
/// what a consumer that assumes its publisher's timing hits, and hits on
/// every tick, which is why `latest_common_time` exists. Unlike the worst
/// case below it touches one buffer.
fn benchmark_not_found_past_newest(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("not_found_past_newest_1k", |b| {
        let (registry, next) = prewarmed_registry(1000);
        let query = Timestamp::from_nanos(next + SAMPLE_INTERVAL_NANOS);

        b.iter(|| black_box(registry.lookup("a", "b", query)).unwrap_err());
    });

    group.finish();
}

/// The cross-time lookup: two legs resolved at different times through the
/// fixed frame.
fn benchmark_lookup_across(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("lookup_across_1k", |b| {
        let (registry, next) = prewarmed_registry(1000);
        let t_new = Timestamp::from_nanos(next - SAMPLE_INTERVAL_NANOS);
        let t_old = Timestamp::from_nanos(BASE_NANOS);

        b.iter(|| black_box(registry.lookup_across("b", t_new, "b", t_old, "a")).unwrap());
    });

    group.finish();
}

/// Builds a 1000-deep static chain "0" -> "1" -> ... -> "1000".
fn deep_static_chain() -> Registry {
    let mut registry = Registry::unbounded();
    for i in 0..1000 {
        registry
            .insert(&static_edge(&i.to_string(), &(i + 1).to_string()))
            .unwrap();
    }
    registry
}

fn benchmark_tree_climb(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k", |b| {
        let registry = deep_static_chain();

        b.iter(|| black_box(registry.lookup("0", "999", Timestamp::from_nanos(0))).unwrap());
    });

    group.finish();
}

/// A failed lookup from the deepest leaf of a 1,000-deep chain to a name
/// no stored edge carries. Names are checked before any chain is walked,
/// so the depth must not show in this number.
fn benchmark_not_found_worst_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("not_found_unknown_frame_1k", |b| {
        let registry = deep_static_chain();

        b.iter(|| {
            black_box(registry.lookup("1000", "unknown", Timestamp::from_nanos(0))).unwrap_err()
        });
    });

    group.finish();
}

fn benchmark_tree_climb_common_parent_elim(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k_common_parent_elim", |b| {
        let mut registry = Registry::unbounded();

        registry.insert(&static_edge("a_999", "b_0")).unwrap();
        registry.insert(&static_edge("a_999", "c_0")).unwrap();

        for i in 0..1000 {
            let next = i + 1;

            for prefix in ["a", "b", "c"] {
                registry
                    .insert(&static_edge(
                        &format!("{prefix}_{i}"),
                        &format!("{prefix}_{next}"),
                    ))
                    .unwrap();
            }
        }

        b.iter(|| black_box(registry.lookup("b_999", "c_999", Timestamp::from_nanos(0))).unwrap());
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_insert,
    benchmark_insert_all,
    benchmark_reparent,
    benchmark_lookup,
    benchmark_lookup_interpolated,
    benchmark_robot_tree,
    benchmark_dynamic_chain_interpolated,
    benchmark_chain_both_directions,
    benchmark_siblings_below_dynamic_ancestors,
    benchmark_latest_common_time,
    benchmark_lookup_across,
    benchmark_tree_climb,
    benchmark_tree_climb_common_parent_elim,
    benchmark_not_found_past_newest,
    benchmark_not_found_worst_case
);

criterion_main!(benches);
