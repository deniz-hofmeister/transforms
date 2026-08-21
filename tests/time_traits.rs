use core::time::Duration;

use transforms::{
    Registry,
    errors::{RegistryError, TimeError},
    geometry::{Quaternion, Transform, Vector3},
    time::{Stamp, TimePoint, Timestamp},
};

/// Builds a `TestTime` transform translated by `x` along the x-axis.
fn test_transform(
    parent: &str,
    child: &str,
    timestamp: Stamp<TestTime>,
    x: f64,
) -> Transform<TestTime> {
    Transform::new(
        parent,
        child,
        Vector3::new(x, 0.0, 0.0),
        Quaternion::identity(),
        timestamp,
    )
    .unwrap()
}

/// A custom nanosecond clock over `u64`, and the complete `TimePoint`
/// implementation an integrator has to write: three methods of pure time
/// arithmetic. No value is reserved for staticness, so the full `u64`
/// range including `0` and `u64::MAX` is ordinary dynamic data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TestTime(u64);

impl TimePoint for TestTime {
    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError> {
        self.0
            .checked_sub(earlier.0)
            .map(Duration::from_nanos)
            .ok_or(TimeError::DurationUnderflow)
    }

    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        let rhs_ns = rhs
            .as_nanos()
            .try_into()
            .map_err(|_| TimeError::DurationOverflow)?;
        self.0
            .checked_sub(rhs_ns)
            .map(Self)
            .ok_or(TimeError::DurationUnderflow)
    }

    fn as_seconds_lossy(self) -> f64 {
        self.0 as f64 / 1_000_000_000.0
    }
}

/// A clock whose instants cannot be expressed as seconds at all — an epoch
/// or a range the conversion does not reach. It borrows `TestTime`'s
/// arithmetic and fails only where the trait allows failure to be expressed:
/// `as_seconds_lossy` is infallible by contract, so it reports `NaN` instead
/// of a plausible-looking number an error message would then quote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct UnconvertibleTime(TestTime);

impl TimePoint for UnconvertibleTime {
    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError> {
        self.0.duration_since(earlier.0)
    }

    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        self.0.checked_sub(rhs).map(Self)
    }

    fn as_seconds_lossy(self) -> f64 {
        f64::NAN
    }
}

#[test]
fn default_timestamp_api_remains_usable() {
    let mut registry = Registry::new();
    let t = Timestamp::from_nanos(1_000_000_000);

    let transform = Transform::new(
        "map",
        "base",
        Vector3::new(1.0, 2.0, 3.0),
        Quaternion::identity(),
        Stamp::At(t),
    )
    .unwrap();

    registry.add_transform(transform.clone()).unwrap();
    let result = registry.get_transform("map", "base", t).unwrap();
    assert_eq!(result, transform);
}

#[cfg(feature = "std")]
#[test]
fn registry_supports_system_time() {
    use std::time::SystemTime;

    let mut registry = Registry::<SystemTime>::with_max_age(Duration::from_secs(10));
    let t0 = SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(1))
        .unwrap();
    let t2 = t0.checked_add(Duration::from_secs(2)).unwrap();
    let t1 = t0.checked_add(Duration::from_secs(1)).unwrap();

    let from = Transform::<SystemTime>::new(
        "a",
        "b",
        Vector3::new(0.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(t0),
    )
    .unwrap();
    let to = Transform::<SystemTime>::new(
        "a",
        "b",
        Vector3::new(2.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::At(t2),
    )
    .unwrap();

    registry.add_transform(from).unwrap();
    registry.add_transform(to).unwrap();

    let mid = registry.get_transform("a", "b", t1).unwrap();
    assert_eq!(mid.timestamp(), Stamp::At(t1));
    assert_eq!(mid.translation(), Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn custom_timestamp_static_transform_is_served_for_any_time() {
    let mut registry = Registry::<TestTime>::new();

    let static_transform = Transform::<TestTime>::new(
        "map",
        "sensor",
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
        Stamp::Static,
    )
    .unwrap();

    registry.add_transform(static_transform.clone()).unwrap();

    let result = registry
        .get_transform("map", "sensor", TestTime(5))
        .unwrap();
    // The static transform is served for any query time, and the result
    // carries the query time.
    assert_eq!(result.translation(), static_transform.translation());
    assert_eq!(result.rotation(), static_transform.rotation());
    assert_eq!(result.timestamp(), Stamp::At(TestTime(5)));
}

#[test]
fn static_between_stamps_a_custom_time_transform_as_static() {
    let mount = Transform::<TestTime>::static_between(
        "map",
        "sensor",
        Vector3::new(1.0, 0.0, 0.0),
        Quaternion::identity(),
    )
    .unwrap();
    assert_eq!(mount.timestamp(), Stamp::Static);
}

#[cfg(feature = "std")]
#[test]
fn system_time_pre_epoch_is_nan_and_epoch_is_ordinary() {
    use std::time::{SystemTime, UNIX_EPOCH};

    // A pre-epoch time point cannot be expressed as seconds since the epoch.
    // `as_seconds_lossy` is infallible by contract, so it reports NaN rather
    // than a plausible-looking number an error message would then quote.
    let pre_epoch = UNIX_EPOCH.checked_sub(Duration::from_secs(1)).unwrap();
    assert!(TimePoint::as_seconds_lossy(pre_epoch).is_nan());

    // UNIX_EPOCH is an ordinary dynamic instant — a zero-initialized wire
    // message stamped at the epoch stays a dynamic sample instead of
    // silently becoming an eternal static transform.
    let mut registry = Registry::<SystemTime>::new();
    registry
        .add_transform(
            Transform::<SystemTime>::new(
                "map",
                "sensor",
                Vector3::new(1.0, 0.0, 0.0),
                Quaternion::identity(),
                Stamp::At(UNIX_EPOCH),
            )
            .unwrap(),
        )
        .unwrap();

    // A single-sample dynamic buffer serves exactly its own instant...
    let result = registry.get_transform("map", "sensor", UNIX_EPOCH).unwrap();
    assert_eq!(result.translation(), Vector3::new(1.0, 0.0, 0.0));
    assert_eq!(result.timestamp(), Stamp::At(UNIX_EPOCH));

    // ...and no other, proving it was not classified static.
    let later = UNIX_EPOCH.checked_add(Duration::from_secs(5)).unwrap();
    assert!(registry.get_transform("map", "sensor", later).is_err());
}

// The remaining tests exercise every static-transform path with a custom
// clock: lookups at range extremes, kind conflicts, mixed chains, eviction,
// and the time-travel lookup. Staticness is a `Stamp` variant, so none of
// this depends on any reserved timestamp value.

#[test]
fn static_lookup_serves_any_time_including_extremes() {
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", Stamp::Static, 1.0))
        .unwrap();

    for probe in [0, 1, 12_345, u64::MAX - 1, u64::MAX] {
        let got = registry.get_transform("a", "b", TestTime(probe)).unwrap();
        assert_eq!(got.translation().x, 1.0);
        // The result carries the requested timestamp.
        assert_eq!(got.timestamp(), Stamp::At(TestTime(probe)));
    }
}

#[test]
fn static_dynamic_conflict_fires_in_both_orders() {
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", Stamp::Static, 1.0))
        .unwrap();
    assert!(matches!(
        registry.add_transform(test_transform("a", "b", Stamp::At(TestTime(5)), 2.0)),
        Err(RegistryError::StaticDynamicConflict)
    ));

    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", Stamp::At(TestTime(5)), 2.0))
        .unwrap();
    assert!(matches!(
        registry.add_transform(test_transform("a", "b", Stamp::Static, 1.0)),
        Err(RegistryError::StaticDynamicConflict)
    ));
}

#[test]
fn range_extremes_are_ordinary_dynamic_values() {
    // No value is reserved: both ends of the clock's range are normal
    // dynamic timestamps.
    for extreme in [0, u64::MAX] {
        let t = TestTime(extreme);
        let mut registry = Registry::<TestTime>::new();
        registry
            .add_transform(test_transform("a", "b", Stamp::At(t), 1.0))
            .unwrap();
        assert_eq!(
            registry.get_transform("a", "b", t).unwrap().translation().x,
            1.0
        );
        // A single-sample dynamic buffer cannot serve other times, proving
        // the buffer was not classified static.
        let other = TestTime(if extreme == 0 { u64::MAX } else { 0 });
        assert!(registry.get_transform("a", "b", other).is_err());
    }
}

#[test]
fn mixed_static_dynamic_chain_interpolates() {
    // a -> b static (x = 1), b -> c dynamic moving x = 0 -> 1 over 10s.
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", Stamp::Static, 1.0))
        .unwrap();
    registry
        .add_transform(test_transform(
            "b",
            "c",
            Stamp::At(TestTime(10_000_000_000)),
            0.0,
        ))
        .unwrap();
    registry
        .add_transform(test_transform(
            "b",
            "c",
            Stamp::At(TestTime(20_000_000_000)),
            1.0,
        ))
        .unwrap();

    let probe = TestTime(15_000_000_000);
    let got = registry.get_transform("a", "c", probe).unwrap();
    assert!((got.translation().x - 1.5).abs() < 1e-12);
    assert_eq!(got.timestamp(), Stamp::At(probe));
    assert_eq!(got.parent(), "a");
    assert_eq!(got.child(), "c");
}

#[test]
fn eviction_spares_the_static_leg() {
    let mut registry = Registry::<TestTime>::with_max_age(Duration::from_secs(10));
    registry
        .add_transform(test_transform("a", "b", Stamp::Static, 1.0))
        .unwrap();
    let t_old = TestTime(100_000_000_000);
    let t_new = TestTime(200_000_000_000);
    registry
        .add_transform(test_transform("b", "c", Stamp::At(t_old), 0.0))
        .unwrap();
    registry
        .add_transform(test_transform("b", "c", Stamp::At(t_new), 5.0))
        .unwrap();

    // The old dynamic sample is evicted; the chain through the static leg
    // still resolves, and the static leg answers at any time on its own.
    assert!(registry.get_transform("b", "c", t_old).is_err());
    let got = registry.get_transform("a", "c", t_new).unwrap();
    assert!((got.translation().x - 6.0).abs() < 1e-12);
    assert_eq!(
        registry
            .get_transform("a", "b", TestTime(0))
            .unwrap()
            .translation()
            .x,
        1.0
    );
}

#[test]
fn a_clock_that_cannot_report_seconds_cannot_mask_the_error() {
    // Error formatting goes through `as_seconds_lossy`, which cannot fail —
    // a conversion error must never replace the error being reported. This
    // clock makes every conversion produce nothing usable; the failure must
    // still arrive intact, and the message must say `NaN` rather than a
    // number a reader would act on.
    let at = |nanos| UnconvertibleTime(TestTime(nanos));
    let sample = |nanos, x| {
        Transform::<UnconvertibleTime>::new(
            "map",
            "sensor",
            Vector3::new(x, 0.0, 0.0),
            Quaternion::identity(),
            Stamp::At(at(nanos)),
        )
        .unwrap()
    };

    let mut registry = Registry::<UnconvertibleTime>::new();
    registry.add_transform(sample(10, 1.0)).unwrap();
    registry.add_transform(sample(20, 2.0)).unwrap();

    let error = registry
        .get_transform("map", "sensor", at(30))
        .expect_err("a query past the covered range must fail");

    // The payload is carried in the caller's own time type, so it survives a
    // conversion that produces no number at all: every instant is still
    // comparable against the clock the caller asked with.
    match &error {
        RegistryError::NotFoundAt {
            frame,
            requested,
            covered,
            ..
        } => {
            assert_eq!(frame, "sensor");
            assert_eq!(*requested, at(30));
            assert_eq!(*covered, Some((at(10), at(20))));
        }
        other => panic!("expected NotFoundAt, got {other:?}"),
    }

    let message = format!("{error}");
    assert!(message.contains("sensor"), "{message}");
    assert!(message.contains("NaN"), "{message}");
}

#[test]
fn time_travel_lookup_works_with_a_custom_clock() {
    // get_transform_at composes legs resolved at different times through a
    // time-agnostic private path; verify the whole flow on a custom clock.
    let t1 = TestTime(1_000_000_000);
    let t2 = TestTime(2_000_000_000);
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("fixed", "a", Stamp::At(t1), 1.0))
        .unwrap();
    registry
        .add_transform(test_transform("fixed", "a", Stamp::At(t2), 2.0))
        .unwrap();
    registry
        .add_transform(test_transform("a", "b", Stamp::At(t1), 0.0))
        .unwrap();

    let result = registry
        .get_transform_at("a", t2, "b", t1, "fixed")
        .unwrap();
    // b sat at fixed-x 1.0 at t1; a is at fixed-x 2.0 at t2, so b expressed
    // in a-at-t2 sits at x = -1.0.
    assert!((result.translation().x - (-1.0)).abs() < 1e-12);
    assert_eq!(result.timestamp(), Stamp::At(t2));
}
