use core::time::Duration;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{cell::Cell, hint::black_box};
use transforms::{
    Registry,
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, Timestamp},
};

/// Base timestamp for dynamic samples.
const BASE_NANOS: u64 = 1_000_000_000;
/// Nanoseconds between consecutive samples in the prepared registries.
const SAMPLE_INTERVAL_NANOS: u64 = 1_000_000;

fn transform_at(
    parent: &str,
    child: &str,
    nanos: u64,
) -> Transform {
    Transform::new(
        parent,
        child,
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(Timestamp::from_nanos(nanos)),
    )
    .unwrap()
}

/// A registry pre-warmed with `samples` dynamic transforms between "a" and
/// "b", spaced `SAMPLE_INTERVAL_NANOS` apart starting at `BASE_NANOS`.
/// Returns the registry and the first free timestamp after the samples.
fn prewarmed_registry(samples: u64) -> (Registry, u64) {
    let mut registry = Registry::new();
    let mut nanos = BASE_NANOS;
    for _ in 0..samples {
        registry
            .add_transform(transform_at("a", "b", nanos))
            .unwrap();
        nanos += SAMPLE_INTERVAL_NANOS;
    }
    (registry, nanos)
}

/// Steady-state insert: the registry is pre-warmed and bounded by `max_age`,
/// each iteration inserts the next sample of the stream. The transform is
/// built in the batch setup so only `add_transform` is measured.
fn benchmark_add_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("add_transform_prewarmed_1k", |b| {
        let mut registry = Registry::with_max_age(Duration::from_secs(1));
        let mut nanos = BASE_NANOS;
        for _ in 0..1000 {
            registry
                .add_transform(transform_at("a", "b", nanos))
                .unwrap();
            nanos += SAMPLE_INTERVAL_NANOS;
        }

        let next = Cell::new(nanos);
        b.iter_batched(
            || {
                let nanos = next.get();
                next.set(nanos + SAMPLE_INTERVAL_NANOS);
                transform_at("a", "b", nanos)
            },
            |transform| registry.add_transform(black_box(transform)).unwrap(),
            BatchSize::SmallInput,
        );
    });

    // 60 s max_age at 1 kHz sample spacing keeps ~60k live entries — the
    // README Quick Start configuration. Guards the eviction path: a return
    // to full-buffer scanning on insert shows up here as a ~100x regression.
    group.bench_function("add_transform_prewarmed_60k", |b| {
        let mut registry = Registry::with_max_age(Duration::from_secs(60));
        let mut nanos = BASE_NANOS;
        for _ in 0..60_000 {
            registry
                .add_transform(transform_at("a", "b", nanos))
                .unwrap();
            nanos += SAMPLE_INTERVAL_NANOS;
        }

        let next = Cell::new(nanos);
        b.iter_batched(
            || {
                let nanos = next.get();
                next.set(nanos + SAMPLE_INTERVAL_NANOS);
                transform_at("a", "b", nanos)
            },
            |transform| registry.add_transform(black_box(transform)).unwrap(),
            BatchSize::SmallInput,
        );
    });

    // 100k resident samples: pins the eviction fix at depth. The old
    // full-map retain scan made this insert O(resident samples), ~1800x
    // slower than the empty-registry case.
    group.bench_function("add_transform_prewarmed_100k", |b| {
        let mut registry = Registry::with_max_age(Duration::from_secs(100));
        let mut nanos = BASE_NANOS;
        for _ in 0..100_000 {
            registry
                .add_transform(transform_at("a", "b", nanos))
                .unwrap();
            nanos += SAMPLE_INTERVAL_NANOS;
        }

        let next = Cell::new(nanos);
        b.iter_batched(
            || {
                let nanos = next.get();
                next.set(nanos + SAMPLE_INTERVAL_NANOS);
                transform_at("a", "b", nanos)
            },
            |transform| registry.add_transform(black_box(transform)).unwrap(),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Lookup at an exactly stored timestamp in a buffer of 1000 samples.
fn benchmark_get_transform(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("get_transform_1k", |b| {
        let (registry, next) = prewarmed_registry(1000);
        let query = Timestamp::from_nanos(next - SAMPLE_INTERVAL_NANOS);

        b.iter(|| black_box(registry.get_transform("a", "b", query)).unwrap());
    });

    group.finish();
}

/// Lookup between two stored samples, forcing interpolation.
fn benchmark_get_transform_interpolated(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("get_transform_interpolated_1k", |b| {
        let (registry, _) = prewarmed_registry(1000);
        let query = Timestamp::from_nanos(
            BASE_NANOS + 500 * SAMPLE_INTERVAL_NANOS + SAMPLE_INTERVAL_NANOS / 2,
        );

        b.iter(|| black_box(registry.get_transform("a", "b", query)).unwrap());
    });

    group.finish();
}

/// A realistic small robot tree mixing static mounts and dynamic edges:
/// map -> odom -> base_link -> {laser, imu, camera -> camera_optical}.
/// The three dynamic edges hold `samples` samples each; the three mounts
/// are static. Deep synthetic chains overstate real-world lookup costs;
/// this is the shape most robots actually query.
fn robot_tree(samples: u64) -> Registry {
    let mut registry = Registry::new();

    registry
        .add_transform(
            Transform::static_between(
                "base_link",
                "laser",
                Vector3::new(0.2, 0.0, 0.1),
                Quaternion::identity(),
            )
            .unwrap(),
        )
        .unwrap();
    registry
        .add_transform(
            Transform::static_between(
                "base_link",
                "imu",
                Vector3::new(0.0, 0.0, 0.05),
                Quaternion::identity(),
            )
            .unwrap(),
        )
        .unwrap();
    registry
        .add_transform(
            Transform::static_between(
                "camera",
                "camera_optical",
                Vector3::zero(),
                Quaternion::identity(),
            )
            .unwrap(),
        )
        .unwrap();

    let mut nanos = BASE_NANOS;
    for _ in 0..samples {
        registry
            .add_transform(transform_at("map", "odom", nanos))
            .unwrap();
        registry
            .add_transform(transform_at("odom", "base_link", nanos))
            .unwrap();
        registry
            .add_transform(transform_at("base_link", "camera", nanos))
            .unwrap();
        nanos += SAMPLE_INTERVAL_NANOS;
    }
    registry
}

/// Interpolating lookups on the realistic robot tree: a full chain from the
/// map into a sensor's optical frame, and a leaf-to-leaf query across the
/// common parent.
fn benchmark_robot_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    let registry = robot_tree(1000);
    let query =
        Timestamp::from_nanos(BASE_NANOS + 500 * SAMPLE_INTERVAL_NANOS + SAMPLE_INTERVAL_NANOS / 2);

    group.bench_function("robot_tree_map_to_camera_optical", |b| {
        b.iter(|| black_box(registry.get_transform("map", "camera_optical", query)).unwrap());
    });
    group.bench_function("robot_tree_leaf_to_leaf", |b| {
        b.iter(|| black_box(registry.get_transform("laser", "camera_optical", query)).unwrap());
    });

    group.finish();
}

/// A 3-hop chain of dynamic edges, each holding 1k samples, queried between
/// samples so every hop interpolates (slerp plus two String clones per hop).
fn benchmark_dynamic_chain_interpolated(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("dynamic_chain_3hop_interpolated_1k", |b| {
        let mut registry = Registry::new();
        let mut nanos = BASE_NANOS;
        for _ in 0..1000 {
            registry
                .add_transform(transform_at("a", "b", nanos))
                .unwrap();
            registry
                .add_transform(transform_at("b", "c", nanos))
                .unwrap();
            registry
                .add_transform(transform_at("c", "d", nanos))
                .unwrap();
            nanos += SAMPLE_INTERVAL_NANOS;
        }
        let query = Timestamp::from_nanos(
            BASE_NANOS + 500 * SAMPLE_INTERVAL_NANOS + SAMPLE_INTERVAL_NANOS / 2,
        );

        b.iter(|| black_box(registry.get_transform("a", "d", query)).unwrap());
    });

    group.finish();
}

/// The time-travel lookup: two legs resolved at different times through the
/// fixed frame, composed via the time-agnostic private path.
fn benchmark_get_transform_at(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("get_transform_at_1k", |b| {
        let (registry, next) = prewarmed_registry(1000);
        let t_new = Timestamp::from_nanos(next - SAMPLE_INTERVAL_NANOS);
        let t_old = Timestamp::from_nanos(BASE_NANOS);

        b.iter(|| black_box(registry.get_transform_at("b", t_new, "b", t_old, "a")).unwrap());
    });

    group.finish();
}

/// A static edge between two frames, with no offset or rotation.
fn static_edge(
    parent: &str,
    child: &str,
) -> Transform {
    Transform::static_between(parent, child, Vector3::zero(), Quaternion::identity()).unwrap()
}

/// Builds a 1000-deep static chain "0" -> "1" -> ... -> "1000".
fn deep_static_chain() -> Registry {
    let mut registry = Registry::new();
    for i in 0..1000 {
        registry
            .add_transform(static_edge(&i.to_string(), &(i + 1).to_string()))
            .unwrap();
    }
    registry
}

fn benchmark_tree_climb(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k", |b| {
        let registry = deep_static_chain();

        b.iter(|| black_box(registry.get_transform("0", "999", Timestamp::zero())).unwrap());
    });

    group.finish();
}

/// Worst-case failed lookup: the walk from the deepest leaf climbs the whole
/// chain to the root, then the diagnosis scans every buffer before the query
/// is reported as `UnknownFrame`.
fn benchmark_not_found_worst_case(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("not_found_unknown_frame_1k", |b| {
        let registry = deep_static_chain();

        b.iter(|| {
            black_box(registry.get_transform("1000", "unknown", Timestamp::zero())).unwrap_err()
        });
    });

    group.finish();
}

fn benchmark_tree_climb_common_parent_elim(c: &mut Criterion) {
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k_common_parent_elim", |b| {
        let mut registry = Registry::new();

        registry.add_transform(static_edge("a_999", "b_0")).unwrap();
        registry.add_transform(static_edge("a_999", "c_0")).unwrap();

        for i in 0..1000 {
            let next = i + 1;

            for prefix in ["a", "b", "c"] {
                registry
                    .add_transform(static_edge(
                        &format!("{prefix}_{i}"),
                        &format!("{prefix}_{next}"),
                    ))
                    .unwrap();
            }
        }

        b.iter(|| black_box(registry.get_transform("b_999", "c_999", Timestamp::zero())).unwrap());
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_add_transform,
    benchmark_get_transform,
    benchmark_get_transform_interpolated,
    benchmark_robot_tree,
    benchmark_dynamic_chain_interpolated,
    benchmark_get_transform_at,
    benchmark_tree_climb,
    benchmark_tree_climb_common_parent_elim,
    benchmark_not_found_worst_case
);

criterion_main!(benches);
