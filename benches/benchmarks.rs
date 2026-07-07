#[cfg(feature = "std")]
use core::time::Duration;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
};

fn create_sample_transform() -> Transform {
    Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        #[cfg(not(feature = "std"))]
        timestamp: Timestamp::zero(),
        #[cfg(feature = "std")]
        timestamp: Timestamp::now(),
        parent: "a".into(),
        child: "b".into(),
    }
}

fn benchmark_transforms(c: &mut Criterion) {
    use transforms::Registry;
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("add_and_get_transform", |b| {
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(feature = "std")]
        let mut registry = Registry::with_max_age(Duration::from_secs(60));
        b.iter(|| {
            let transform = create_sample_transform();
            let t = transform.timestamp;
            registry.add_transform(transform).unwrap();
            let _ = black_box(registry.get_transform("a", "b", t));
        });
    });

    group.finish();
}

fn benchmark_transforms_with_preparation(c: &mut Criterion) {
    use transforms::Registry;
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("add_and_get_transform_1k", |b| {
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(feature = "std")]
        let mut registry = Registry::with_max_age(Duration::from_secs(60));

        // Prepare registry with 1000 transforms
        for _ in 0..1000 {
            let transform = create_sample_transform();
            registry.add_transform(transform).unwrap();
        }

        b.iter(|| {
            let transform = create_sample_transform();
            let t = transform.timestamp;
            registry.add_transform(transform).unwrap();
            let _ = black_box(registry.get_transform("a", "b", t));
        });
    });

    group.finish();
}

fn benchmark_tree_climb(c: &mut Criterion) {
    use transforms::Registry;
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k", |b| {
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(feature = "std")]
        let mut registry = Registry::with_max_age(Duration::from_secs(60));

        // Prepare registry with 1000 transforms
        for i in 0..1000 {
            let mut transform = Transform::identity();
            transform.parent = i.to_string();
            transform.child = (i + 1).to_string();
            registry.add_transform(transform).unwrap();
        }

        b.iter(|| {
            let _ = black_box(registry.get_transform("0", "999", Timestamp::zero()));
        });
    });

    group.finish();
}

fn benchmark_tree_climb_common_parent_elim(c: &mut Criterion) {
    use transforms::Registry;
    let mut group = c.benchmark_group("benchmark");
    group.sample_size(1000);

    group.bench_function("tree_climb_1k_common_parent_elim", |b| {
        #[cfg(not(feature = "std"))]
        let mut registry = Registry::new();
        #[cfg(feature = "std")]
        let mut registry = Registry::with_max_age(Duration::from_secs(60));

        // Prepare registry with 1000 transforms
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

        b.iter(|| {
            let _ = black_box(registry.get_transform("b_999", "c_999", Timestamp::zero()));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_transforms,
    benchmark_transforms_with_preparation,
    benchmark_tree_climb,
    benchmark_tree_climb_common_parent_elim
);

criterion_main!(benches);
