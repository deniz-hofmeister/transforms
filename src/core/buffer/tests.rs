#[cfg(test)]
mod buffer_tests {
    use crate::{
        core::{Buffer, buffer::BufferError},
        geometry::{Quaternion, Transform, Vector3},
        time::Timestamp,
    };
    use core::time::Duration;

    fn create_transform(t: Timestamp) -> Transform {
        let translation = Vector3::new(1.0, 2.0, 3.0);
        let rotation = Quaternion::identity();
        let timestamp = t;
        let parent = "map".into();
        let child = "base".into();
        Transform {
            translation,
            rotation,
            timestamp,
            parent,
            child,
        }
    }

    #[test]
    fn insert_and_get() {
        #[cfg(not(feature = "std"))]
        let mut buffer = Buffer::new();
        #[cfg(not(feature = "std"))]
        let t = (Timestamp::zero() + Duration::from_secs(1)).unwrap();

        #[cfg(feature = "std")]
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = (Timestamp::now() + Duration::from_secs(1)).unwrap();

        let transform = create_transform(t);
        buffer.insert(transform.clone()).unwrap();

        let mut r = buffer.get(&transform.timestamp);

        assert!(r.is_ok(), "expected transform, got {r:?}");
        assert_eq!(r.unwrap(), transform);

        r = buffer.get(&(transform.timestamp + Duration::from_secs(1)).unwrap());
        assert!(r.is_err(), "expected no transform, got {r:?}");

        r = buffer.get(&(transform.timestamp - Duration::from_secs(1)).unwrap());
        assert!(r.is_err(), "expected no transform, got {r:?}");
    }

    #[test]
    fn insert_and_get_static() {
        #[cfg(not(feature = "std"))]
        let mut buffer = Buffer::new();

        #[cfg(feature = "std")]
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));

        let t = Timestamp::zero();
        let transform = create_transform(t);

        buffer.insert(transform.clone()).unwrap();

        let mut r = buffer.get(&(transform.timestamp + Duration::from_secs(1)).unwrap());

        assert!(r.is_ok(), "expected transform, got {r:?}");
        assert_eq!(r.unwrap(), transform);

        r = buffer.get(&(transform.timestamp + Duration::from_secs(2)).unwrap());
        assert!(r.is_ok(), "expected transform, got {r:?}");
        assert_eq!(r.unwrap(), transform);
    }

    #[test]
    fn get_nearest() {
        #[cfg(not(feature = "std"))]
        let mut buffer = Buffer::new();
        #[cfg(not(feature = "std"))]
        let t = (Timestamp::zero() + Duration::from_secs(1)).unwrap();

        #[cfg(feature = "std")]
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = (Timestamp::now() + Duration::from_secs(1)).unwrap();

        let p1 = create_transform((t + Duration::from_secs(1)).unwrap());
        let p2 = create_transform((t + Duration::from_secs(2)).unwrap());
        let p3 = create_transform((t + Duration::from_secs(3)).unwrap());

        buffer.insert(p1.clone()).unwrap();
        buffer.insert(p2.clone()).unwrap();
        buffer.insert(p3.clone()).unwrap();

        // Exact match
        let (before, after) = buffer.get_nearest(&p2.timestamp);
        assert_eq!(before.unwrap(), (&p2.timestamp, &p2));
        assert_eq!(after.unwrap(), (&p2.timestamp, &p2));

        // Between two points
        let p_mid = (p1.timestamp + Duration::from_millis(500)).unwrap();
        let (before, after) = buffer.get_nearest(&p_mid);
        assert_eq!(before.unwrap(), (&p1.timestamp, &p1));
        assert_eq!(after.unwrap(), (&p2.timestamp, &p2));

        // Before first point
        let p_0 = (p1.timestamp - Duration::from_secs(1)).unwrap();
        let (before, after) = buffer.get_nearest(&p_0);
        assert_eq!(before, None);
        assert_eq!(after.unwrap(), (&p1.timestamp, &p1));

        // After last point
        let p_4 = (p3.timestamp + Duration::from_secs(1)).unwrap();
        let (before, after) = buffer.get_nearest(&p_4);
        assert_eq!(before.unwrap(), (&p3.timestamp, &p3));
        assert_eq!(after, None);

        // Exactly at first point
        let (before, after) = buffer.get_nearest(&p1.timestamp);
        assert_eq!(before.unwrap(), (&p1.timestamp, &p1));
        assert_eq!(after.unwrap(), (&p1.timestamp, &p1));

        // Exactly at last point
        let (before, after) = buffer.get_nearest(&p3.timestamp);
        assert_eq!(before.unwrap(), (&p3.timestamp, &p3));
        assert_eq!(after.unwrap(), (&p3.timestamp, &p3));
    }

    #[test]
    fn empty_buffer() {
        #[cfg(not(feature = "std"))]
        let buffer = Buffer::new();

        #[cfg(feature = "std")]
        let buffer = Buffer::with_max_age(Duration::from_secs(10));

        assert!(buffer.get(&Timestamp::from_nanos(1000)).is_err());

        let (before, after) = buffer.get_nearest(&Timestamp::from_nanos(1000));
        assert!(before.is_none());
        assert!(after.is_none());
    }

    #[test]
    #[cfg(not(feature = "std"))]
    fn delete_before() {
        let mut buffer = Buffer::new();
        let t = (Timestamp::zero() + Duration::from_secs(1)).unwrap();

        let p1 = create_transform(t);
        let p2 = create_transform((t + Duration::from_secs(2)).unwrap());

        buffer.insert(p1.clone()).unwrap();
        buffer.insert(p2.clone()).unwrap();

        assert!(buffer.get(&p1.timestamp).is_ok());
        assert!(buffer.get(&p2.timestamp).is_ok());

        buffer.delete_before(Timestamp::from_nanos(2_000_000_000));

        assert!(buffer.get(&p1.timestamp).is_err());
        assert!(buffer.get(&p2.timestamp).is_ok());
    }

    #[test]
    #[cfg(feature = "std")]
    fn delete_expired() {
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
        let t = (Timestamp::now() + Duration::from_secs(1)).unwrap();

        let p1 = create_transform(t);
        let p2 = create_transform((t + Duration::from_secs(1)).unwrap());
        let p3 = create_transform((t + Duration::from_secs(2)).unwrap());

        buffer.insert(p1.clone()).unwrap();
        buffer.insert(p2.clone()).unwrap();
        buffer.insert(p3.clone()).unwrap();

        let get_1 = buffer.get(&(t - Duration::from_secs(2)).unwrap());
        let get_2 = buffer.get(&(t - Duration::from_secs(1)).unwrap());
        let get_3 = buffer.get(&t);

        assert!(get_1.is_err());
        // The following is not found because by this time, it has expired.
        assert!(get_2.is_err());
        assert!(get_3.is_ok());
    }

    #[test]
    fn single_point_buffer() {
        #[cfg(not(feature = "std"))]
        let mut buffer = Buffer::new();
        #[cfg(not(feature = "std"))]
        let t = (Timestamp::zero() + Duration::from_secs(1)).unwrap();

        #[cfg(feature = "std")]
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
        #[cfg(feature = "std")]
        let t = (Timestamp::now() + Duration::from_secs(1)).unwrap();

        let point = create_transform(t);
        buffer.insert(point.clone()).unwrap();

        // Before the point
        let (before, after) = buffer.get_nearest(&(t - Duration::from_secs(1)).unwrap());
        assert!(before.is_none());
        assert_eq!(after.unwrap(), (&point.timestamp, &point));

        // Exact match
        let (before, after) = buffer.get_nearest(&t);
        assert_eq!(before.unwrap(), (&point.timestamp, &point));
        assert_eq!(after.unwrap(), (&point.timestamp, &point));

        // After the point
        let (before, after) = buffer.get_nearest(&(t + Duration::from_secs(1)).unwrap());
        assert_eq!(before.unwrap(), (&point.timestamp, &point));
        assert!(after.is_none());
    }

    #[test]
    fn insert_rejects_static_dynamic_mixing() {
        #[cfg(not(feature = "std"))]
        let t_dynamic = (Timestamp::zero() + Duration::from_secs(1)).unwrap();
        #[cfg(feature = "std")]
        let t_dynamic = Timestamp::now();

        let static_tf = create_transform(Timestamp::zero());
        let dynamic_tf = create_transform(t_dynamic);

        // Static first, then dynamic.
        #[cfg(not(feature = "std"))]
        let mut buffer = Buffer::new();
        #[cfg(feature = "std")]
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));

        buffer.insert(static_tf.clone()).unwrap();
        assert!(matches!(
            buffer.insert(dynamic_tf.clone()),
            Err(BufferError::StaticDynamicConflict)
        ));

        // The static transform is still served after the rejected insert.
        assert_eq!(buffer.get(&t_dynamic).unwrap(), static_tf);

        // Dynamic first, then static.
        #[cfg(not(feature = "std"))]
        let mut buffer = Buffer::new();
        #[cfg(feature = "std")]
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));

        buffer.insert(dynamic_tf.clone()).unwrap();
        assert!(matches!(
            buffer.insert(static_tf),
            Err(BufferError::StaticDynamicConflict)
        ));

        // The dynamic transform is still served after the rejected insert.
        assert_eq!(buffer.get(&t_dynamic).unwrap(), dynamic_tf);
    }

    #[test]
    fn delete_before_preserves_static_transforms() {
        let mut buffer: Buffer = Buffer::new();

        let static_tf = create_transform(Timestamp::zero());
        buffer.insert(static_tf.clone()).unwrap();

        // Manual cleanup with any cutoff must not destroy a static transform:
        // it is valid for all time, not just before the cutoff.
        buffer.delete_before(Timestamp::from_nanos(5_000_000_000));

        assert_eq!(
            buffer.get(&Timestamp::from_nanos(9_000_000_000)).unwrap(),
            static_tf,
            "static transforms must survive manual cleanup"
        );
    }

    #[test]
    fn insert_rejects_invalid_transforms() {
        use crate::errors::TransformError;

        let mut buffer: Buffer = Buffer::new();

        let t = Timestamp::from_nanos(1_000_000_000);

        // A rotation-equivalent but non-unit quaternion would silently scale
        // every lookup it takes part in.
        let mut non_unit = create_transform(t);
        non_unit.rotation = Quaternion::new(2.0, 0.0, 0.0, 0.0);
        assert!(matches!(
            buffer.insert(non_unit),
            Err(BufferError::TransformError(
                TransformError::NonUnitRotation(_)
            ))
        ));

        let mut non_finite = create_transform(t);
        non_finite.translation = Vector3::new(f64::NAN, 0.0, 0.0);
        assert!(matches!(
            buffer.insert(non_finite),
            Err(BufferError::TransformError(TransformError::NonFiniteValues))
        ));

        // Unit-norm rotations with f32-grade precision loss must be accepted.
        let mut f32_grade = create_transform(t);
        f32_grade.rotation = Quaternion::new(1.0 + 1e-8, 0.0, 0.0, 0.0);
        assert!(buffer.insert(f32_grade).is_ok());
    }
}
