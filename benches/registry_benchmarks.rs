use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};

fn create_sample_transform() -> Transform {
    Transform {
        translation: Vector3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        rotation: Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        timestamp: Timestamp::now(),
        parent: "a".to_string(),
        child: "b".to_string(),
    }
}

fn benchmark_sync_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync");
    group.sample_size(1000);

    group.bench_function("add_and_get_transform", |b| {
        let mut registry = Registry::new(Duration::from_secs(60));
        b.iter(|| {
            let transform = create_sample_transform();
            let t = transform.timestamp;
            let _ = black_box(registry.add_transform(transform));
            let _ = black_box(registry.get_transform("a", "b", t));
        });
    });

    group.finish();
}

fn benchmark_sync_transforms_with_preparation(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync");
    group.sample_size(1000);

    group.bench_function("add_and_get_transform_1k", |b| {
        let mut registry = Registry::new(Duration::from_secs(60));

        // Prepare registry with 1000 transforms
        for _ in 0..1000 {
            let transform = create_sample_transform();
            let _ = registry.add_transform(transform);
        }

        b.iter(|| {
            let transform = create_sample_transform();
            let t = transform.timestamp;
            let _ = black_box(registry.add_transform(transform));
            let _ = black_box(registry.get_transform("a", "b", t));
        });
    });

    group.finish();
}

fn benchmark_sync_tree_climb(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k", |b| {
        let mut registry = Registry::new(Duration::from_secs(60));

        // Prepare registry with 1000 transforms
        for i in 0..1000 {
            let mut transform = Transform::identity();
            transform.parent = i.to_string();
            transform.child = (i + 1).to_string();
            let _ = registry.add_transform(transform);
        }

        b.iter(|| {
            let _ = black_box(registry.get_transform("0", "999", Timestamp::zero()));
        });
    });

    group.finish();
}

fn benchmark_sync_tree_climb_common_parent_elim(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k_common_parent_elim", |b| {
        let mut registry = Registry::new(Duration::from_secs(60));

        // Prepare registry with 1000 transforms
        let mut transform = Transform::identity();
        transform.parent = "a_999".to_string();
        transform.child = "b_0".to_string();
        let _ = registry.add_transform(transform);

        let mut transform = Transform::identity();
        transform.parent = "a_999".to_string();
        transform.child = "c_0".to_string();
        let _ = registry.add_transform(transform);

        for i in 0..1000 {
            let mut transform = Transform::identity();
            transform.parent = "a_".to_string() + &i.to_string();
            transform.child = "a_".to_string() + &(i + 1).to_string();
            let _ = registry.add_transform(transform);

            let mut transform = Transform::identity();
            transform.parent = "b_".to_string() + &i.to_string();
            transform.child = "b_".to_string() + &(i + 1).to_string();
            let _ = registry.add_transform(transform);

            let mut transform = Transform::identity();
            transform.parent = "c_".to_string() + &i.to_string();
            transform.child = "c_".to_string() + &(i + 1).to_string();
            let _ = registry.add_transform(transform);
        }

        b.iter(|| {
            let _ = black_box(registry.get_transform("b_999", "c_999", Timestamp::zero()));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_sync_transforms,
    benchmark_sync_transforms_with_preparation,
    benchmark_sync_tree_climb,
    benchmark_sync_tree_climb_common_parent_elim
);

criterion_main!(benches);
