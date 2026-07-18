use core::time::Duration;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::{cell::Cell, hint::black_box};
use transforms::{
    Registry,
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
};

/// Base timestamp for dynamic samples; `t = 0` is the static sentinel.
const BASE_NANOS: u128 = 1_000_000_000;
/// Nanoseconds between consecutive samples in the prepared registries.
const SAMPLE_INTERVAL_NANOS: u128 = 1_000_000;

fn transform_at(
    parent: &str,
    child: &str,
    nanos: u128,
) -> Transform {
    Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: Timestamp::from_nanos(nanos),
        parent: parent.into(),
        child: child.into(),
    }
}

/// A registry pre-warmed with `samples` dynamic transforms between "a" and
/// "b", spaced `SAMPLE_INTERVAL_NANOS` apart starting at `BASE_NANOS`.
/// Returns the registry and the first free timestamp after the samples.
fn prewarmed_registry(samples: u128) -> (Registry, u128) {
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

/// Builds a 1000-deep static chain "0" -> "1" -> ... -> "1000".
fn deep_static_chain() -> Registry {
    let mut registry = Registry::new();
    for i in 0..1000 {
        let mut transform = Transform::identity();
        transform.parent = i.to_string();
        transform.child = (i + 1).to_string();
        registry.add_transform(transform).unwrap();
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

        let mut transform = Transform::identity();
        transform.parent = "a_999".into();
        transform.child = "b_0".into();
        registry.add_transform(transform).unwrap();

        let mut transform = Transform::identity();
        transform.parent = "a_999".into();
        transform.child = "c_0".into();
        registry.add_transform(transform).unwrap();

        for i in 0..1000 {
            let next = i + 1;

            let mut transform = Transform::identity();
            transform.parent = format!("a_{i}");
            transform.child = format!("a_{next}");
            registry.add_transform(transform).unwrap();

            let mut transform = Transform::identity();
            transform.parent = format!("b_{i}");
            transform.child = format!("b_{next}");
            registry.add_transform(transform).unwrap();

            let mut transform = Transform::identity();
            transform.parent = format!("c_{i}");
            transform.child = format!("c_{next}");
            registry.add_transform(transform).unwrap();
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
    benchmark_tree_climb,
    benchmark_tree_climb_common_parent_elim,
    benchmark_not_found_worst_case
);

criterion_main!(benches);
