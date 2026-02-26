use core::time::Duration;

use transforms::{
    errors::TimestampError,
    geometry::{Quaternion, Transform, Vector3},
    time::{Timestamp, TimestampLike},
    Registry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TestTime(u64);

impl TimestampLike for TestTime {
    fn static_timestamp() -> Self {
        Self(u64::MAX)
    }

    fn duration_since(
        self,
        earlier: Self,
    ) -> Result<Duration, TimestampError> {
        self.0
            .checked_sub(earlier.0)
            .map(Duration::from_nanos)
            .ok_or(TimestampError::DurationUnderflow)
    }

    fn checked_add(
        self,
        rhs: Duration,
    ) -> Result<Self, TimestampError> {
        let rhs_ns = rhs
            .as_nanos()
            .try_into()
            .map_err(|_| TimestampError::DurationOverflow)?;
        self.0
            .checked_add(rhs_ns)
            .map(Self)
            .ok_or(TimestampError::DurationOverflow)
    }

    fn checked_sub(
        self,
        rhs: Duration,
    ) -> Result<Self, TimestampError> {
        let rhs_ns = rhs
            .as_nanos()
            .try_into()
            .map_err(|_| TimestampError::DurationOverflow)?;
        self.0
            .checked_sub(rhs_ns)
            .map(Self)
            .ok_or(TimestampError::DurationUnderflow)
    }

    fn as_seconds(self) -> Result<f64, TimestampError> {
        Ok(self.0 as f64 / 1_000_000_000.0)
    }
}

#[test]
fn default_timestamp_api_remains_usable() {
    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    let transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "map".into(),
        child: "base".into(),
    };

    registry.add_transform(transform.clone());
    let result = registry.get_transform("map", "base", t).unwrap();
    assert_eq!(result, transform);
}

#[cfg(feature = "std")]
#[test]
fn registry_supports_system_time() {
    use std::time::SystemTime;

    let mut registry = Registry::<SystemTime>::new(Duration::from_secs(10));
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

    registry.add_transform(from);
    registry.add_transform(to);

    let mid = registry.get_transform("a", "b", t1).unwrap();
    assert_eq!(mid.timestamp, t1);
    assert_eq!(mid.translation, Vector3::new(1.0, 0.0, 0.0));
}

#[test]
fn custom_timestamp_static_policy_is_respected() {
    #[cfg(not(feature = "std"))]
    let mut registry = Registry::<TestTime>::new();
    #[cfg(feature = "std")]
    let mut registry = Registry::<TestTime>::new(Duration::from_secs(10));

    let static_transform = Transform::<TestTime> {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: TestTime::static_timestamp(),
        parent: "map".into(),
        child: "sensor".into(),
    };

    registry.add_transform(static_transform.clone());

    let result = registry
        .get_transform("map", "sensor", TestTime(5))
        .unwrap();
    assert_eq!(result, static_transform);
}

#[test]
fn identity_uses_custom_static_timestamp() {
    let identity = Transform::<TestTime>::identity();
    assert_eq!(identity.timestamp, TestTime::static_timestamp());
}
