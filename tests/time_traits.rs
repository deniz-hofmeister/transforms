use core::time::Duration;

use transforms::{
    Registry,
    errors::{BufferError, TimeError},
    geometry::{Quaternion, Transform, Vector3},
    time::{TimePoint, Timestamp},
};

/// Builds a `TestTime` transform translated by `x` along the x-axis.
fn test_transform(
    parent: &str,
    child: &str,
    timestamp: TestTime,
    x: f64,
) -> Transform<TestTime> {
    Transform {
        translation: Vector3::new(x, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp,
        parent: parent.into(),
        child: child.into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TestTime(u64);

impl TimePoint for TestTime {
    fn static_timestamp() -> Self {
        Self(u64::MAX)
    }

    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimeError> {
        self.0
            .checked_sub(earlier.0)
            .map(Duration::from_nanos)
            .ok_or(TimeError::DurationUnderflow)
    }

    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimeError> {
        let rhs_ns = rhs
            .as_nanos()
            .try_into()
            .map_err(|_| TimeError::DurationOverflow)?;
        self.0
            .checked_add(rhs_ns)
            .map(Self)
            .ok_or(TimeError::DurationOverflow)
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

    fn as_seconds(self) -> Result<f64, TimeError> {
        Ok(self.0 as f64 / 1_000_000_000.0)
    }
}

#[test]
fn default_timestamp_api_remains_usable() {
    let mut registry = Registry::new();
    let t = Timestamp::from_nanos(1_000_000_000);

    let transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "map".into(),
        child: "base".into(),
    };

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

    let from = Transform::<SystemTime> {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t0,
        parent: "a".into(),
        child: "b".into(),
    };
    let to = Transform::<SystemTime> {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t2,
        parent: "a".into(),
        child: "b".into(),
    };

    registry.add_transform(from).unwrap();
    registry.add_transform(to).unwrap();

    let mid = registry.get_transform("a", "b", t1).unwrap();
    assert_eq!(mid.timestamp, t1);
    assert_eq!(mid.translation, Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn custom_timestamp_static_policy_is_respected() {
    let mut registry = Registry::<TestTime>::new();

    let static_transform = Transform::<TestTime> {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: TestTime::static_timestamp(),
        parent: "map".into(),
        child: "sensor".into(),
    };

    registry.add_transform(static_transform.clone()).unwrap();

    let result = registry
        .get_transform("map", "sensor", TestTime(5))
        .unwrap();
    // The static transform is served for any query time, and the result
    // carries the query time rather than the custom static sentinel.
    assert_eq!(result.translation, static_transform.translation);
    assert_eq!(result.rotation, static_transform.rotation);
    assert_eq!(result.timestamp, TestTime(5));
}

#[test]
fn identity_uses_custom_static_timestamp() {
    let identity = Transform::<TestTime>::identity();
    assert_eq!(identity.timestamp, TestTime::static_timestamp());
}

#[cfg(feature = "std")]
#[test]
fn system_time_pre_epoch_errs_and_epoch_is_static() {
    use std::time::{SystemTime, UNIX_EPOCH};

    // A pre-epoch time point cannot be expressed as seconds since the epoch;
    // the checked conversion must err and the lossy one must yield NaN.
    let pre_epoch = UNIX_EPOCH.checked_sub(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        TimePoint::as_seconds(pre_epoch),
        Err(TimeError::DurationUnderflow)
    ));
    assert!(TimePoint::as_seconds_lossy(pre_epoch).is_nan());
    assert!(!pre_epoch.is_static());

    // UNIX_EPOCH is the static sentinel for SystemTime: a transform stored
    // there is served for any query time, and the result carries the query
    // time.
    assert!(UNIX_EPOCH.is_static());

    let mut registry = Registry::<SystemTime>::new();
    registry
        .add_transform(Transform::<SystemTime> {
            translation: Vector3::new(1.0, 0.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: UNIX_EPOCH,
            parent: "map".into(),
            child: "sensor".into(),
        })
        .unwrap();

    let query = UNIX_EPOCH.checked_add(Duration::from_secs(5)).unwrap();
    let result = registry.get_transform("map", "sensor", query).unwrap();
    assert_eq!(result.translation, Vector3::new(1.0, 0.0, 0.0));
    assert_eq!(result.timestamp, query);
}

// The buffer docs suggest `u64::MAX` as an alternative static sentinel;
// `TestTime` uses exactly that. These tests lock in that every
// sentinel-dependent path works with a MAX-valued sentinel, where the
// sentinel is the largest value instead of the smallest.

#[test]
fn max_sentinel_static_lookup_serves_any_time_including_extremes() {
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", TestTime::static_timestamp(), 1.0))
        .unwrap();

    for probe in [0, 1, 12_345, u64::MAX - 1, u64::MAX] {
        let got = registry.get_transform("a", "b", TestTime(probe)).unwrap();
        assert_eq!(got.translation.x, 1.0);
        // The result carries the requested timestamp, not the sentinel.
        assert_eq!(got.timestamp, TestTime(probe));
    }
}

#[test]
fn max_sentinel_static_dynamic_conflict_fires_in_both_orders() {
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", TestTime::static_timestamp(), 1.0))
        .unwrap();
    assert!(matches!(
        registry.add_transform(test_transform("a", "b", TestTime(5), 2.0)),
        Err(BufferError::StaticDynamicConflict)
    ));

    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", TestTime(5), 2.0))
        .unwrap();
    assert!(matches!(
        registry.add_transform(test_transform("a", "b", TestTime::static_timestamp(), 1.0,)),
        Err(BufferError::StaticDynamicConflict)
    ));
}

#[test]
fn max_sentinel_near_max_value_is_ordinary_dynamic() {
    // The sentinel check is equality-only: u64::MAX - 1 is a normal
    // dynamic timestamp even though it is adjacent to the sentinel.
    let t = TestTime(u64::MAX - 1);
    assert!(!t.is_static());

    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", t, 1.0))
        .unwrap();
    assert_eq!(
        registry.get_transform("a", "b", t).unwrap().translation.x,
        1.0
    );
    // A single-sample dynamic buffer cannot serve other times, proving the
    // buffer was not classified static.
    assert!(registry.get_transform("a", "b", TestTime(0)).is_err());
}

#[test]
fn max_sentinel_mixed_static_dynamic_chain_interpolates() {
    // a -> b static (x = 1), b -> c dynamic moving x = 0 -> 1 over 10s.
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("a", "b", TestTime::static_timestamp(), 1.0))
        .unwrap();
    registry
        .add_transform(test_transform("b", "c", TestTime(10_000_000_000), 0.0))
        .unwrap();
    registry
        .add_transform(test_transform("b", "c", TestTime(20_000_000_000), 1.0))
        .unwrap();

    let probe = TestTime(15_000_000_000);
    let got = registry.get_transform("a", "c", probe).unwrap();
    assert!((got.translation.x - 1.5).abs() < 1e-12);
    assert_eq!(got.timestamp, probe);
    assert_eq!(got.parent, "a");
    assert_eq!(got.child, "c");
}

#[test]
fn max_sentinel_eviction_spares_the_static_leg() {
    let mut registry = Registry::<TestTime>::with_max_age(Duration::from_secs(10));
    registry
        .add_transform(test_transform("a", "b", TestTime::static_timestamp(), 1.0))
        .unwrap();
    let t_old = TestTime(100_000_000_000);
    let t_new = TestTime(200_000_000_000);
    registry
        .add_transform(test_transform("b", "c", t_old, 0.0))
        .unwrap();
    registry
        .add_transform(test_transform("b", "c", t_new, 5.0))
        .unwrap();

    // The old dynamic sample is evicted; the chain through the static leg
    // still resolves, and the static leg answers at any time on its own.
    assert!(registry.get_transform("b", "c", t_old).is_err());
    let got = registry.get_transform("a", "c", t_new).unwrap();
    assert!((got.translation.x - 6.0).abs() < 1e-12);
    assert_eq!(
        registry
            .get_transform("a", "b", TestTime(0))
            .unwrap()
            .translation
            .x,
        1.0
    );
}

#[test]
fn max_sentinel_time_travel_lookup_works() {
    // get_transform_at composes legs resolved at different times through a
    // time-agnostic private path; verify the whole flow with a MAX sentinel.
    let t1 = TestTime(1_000_000_000);
    let t2 = TestTime(2_000_000_000);
    let mut registry = Registry::<TestTime>::new();
    registry
        .add_transform(test_transform("fixed", "a", t1, 1.0))
        .unwrap();
    registry
        .add_transform(test_transform("fixed", "a", t2, 2.0))
        .unwrap();
    registry
        .add_transform(test_transform("a", "b", t1, 0.0))
        .unwrap();

    let result = registry
        .get_transform_at("a", t2, "b", t1, "fixed")
        .unwrap();
    // b sat at fixed-x 1.0 at t1; a is at fixed-x 2.0 at t2, so b expressed
    // in a-at-t2 sits at x = -1.0.
    assert!((result.translation.x - (-1.0)).abs() < 1e-12);
    assert_eq!(result.timestamp, t2);
}
