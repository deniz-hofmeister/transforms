use core::time::Duration;

use transforms::{
    Registry,
    errors::TimeError,
    geometry::{Quaternion, Transform, Vector3},
    time::{TimePoint, Timestamp},
};

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
