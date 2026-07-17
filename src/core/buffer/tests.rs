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
        let mut buffer = Buffer::new();
        let t = Timestamp::from_nanos(1_000_000_000);

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
        let mut buffer = Buffer::new();

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
        let mut buffer = Buffer::new();
        let t = Timestamp::from_nanos(1_000_000_000);

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
        let buffer = Buffer::new();

        assert!(buffer.get(&Timestamp::from_nanos(1000)).is_err());

        let (before, after) = buffer.get_nearest(&Timestamp::from_nanos(1000));
        assert!(before.is_none());
        assert!(after.is_none());
    }

    #[test]
    fn delete_before() {
        let mut buffer = Buffer::new();
        let t = Timestamp::from_nanos(1_000_000_000);

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
    fn delete_expired() {
        let mut buffer = Buffer::with_max_age(Duration::from_secs(10));
        let t = Timestamp::from_nanos(20_000_000_000);

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
        // Before the earliest stored sample: nothing to interpolate from.
        assert!(get_2.is_err());
        assert!(get_3.is_ok());
    }

    #[test]
    fn single_point_buffer() {
        let mut buffer = Buffer::new();
        let t = Timestamp::from_nanos(1_000_000_000);

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
        let t_dynamic = Timestamp::from_nanos(1_000_000_000);

        let static_tf = create_transform(Timestamp::zero());
        let dynamic_tf = create_transform(t_dynamic);

        // Static first, then dynamic.
        let mut buffer = Buffer::new();

        buffer.insert(static_tf.clone()).unwrap();
        assert!(matches!(
            buffer.insert(dynamic_tf.clone()),
            Err(BufferError::StaticDynamicConflict)
        ));

        // The static transform is still served after the rejected insert.
        assert_eq!(buffer.get(&t_dynamic).unwrap(), static_tf);

        // Dynamic first, then static.
        let mut buffer = Buffer::new();

        buffer.insert(dynamic_tf.clone()).unwrap();
        assert!(matches!(
            buffer.insert(static_tf),
            Err(BufferError::StaticDynamicConflict)
        ));

        // The dynamic transform is still served after the rejected insert.
        assert_eq!(buffer.get(&t_dynamic).unwrap(), dynamic_tf);
    }

    #[test]
    fn insert_expires_entries_older_than_max_age() {
        let mut buffer = Buffer::with_max_age(Duration::from_secs(1));

        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(6_000_000_000);

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t2)).unwrap();

        // t1 is more than max_age older than the latest inserted timestamp,
        // so it must have been expired by the second insert.
        assert!(
            buffer.get(&t1).is_err(),
            "entry older than max_age must expire on insert"
        );
        assert!(buffer.get(&t2).is_ok());
    }

    #[test]
    fn new_buffer_never_expires_entries() {
        let mut buffer: Buffer = Buffer::new();

        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_600_000_000_000);

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t2)).unwrap();

        assert!(
            buffer.get(&t1).is_ok(),
            "Buffer::new must not expire entries"
        );
        assert!(buffer.get(&t2).is_ok());
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

    #[test]
    fn frame_accessors_reflect_pinning() {
        let mut buffer = Buffer::new();
        assert_eq!(buffer.parent(), None);
        assert_eq!(buffer.child(), None);

        let t = Timestamp::from_nanos(1_000_000_000);
        buffer.insert(create_transform(t)).unwrap();
        assert_eq!(buffer.parent(), Some("map"));
        assert_eq!(buffer.child(), Some("base"));

        // The pins survive the buffer being emptied, matching the documented
        // parent behavior: dropping the buffer is the only release.
        buffer.delete_before((t + Duration::from_secs(1)).unwrap());
        assert!(buffer.is_empty());
        assert_eq!(buffer.parent(), Some("map"));
        assert_eq!(buffer.child(), Some("base"));
    }

    #[test]
    fn insert_rejects_child_frame_mismatch_static() {
        let mut buffer = Buffer::new();

        // Static calibration transform for map -> base.
        let original = create_transform(Timestamp::zero());
        buffer.insert(original.clone()).unwrap();

        // Same parent, different child (a frame-naming bug): without child
        // pinning this key collision silently overwrote the stored data.
        let mut other = create_transform(Timestamp::zero());
        other.child = "lidar".into();
        other.translation = Vector3::new(9.0, 9.0, 9.0);
        let result = buffer.insert(other);
        assert!(
            matches!(result, Err(BufferError::ChildFrameMismatch(ref pinned)) if pinned == "base"),
            "expected ChildFrameMismatch, got {result:?}"
        );

        // The original static transform must be untouched and retrievable.
        assert_eq!(
            buffer.get(&Timestamp::from_nanos(1_000_000_000)).unwrap(),
            original,
            "the pinned child's static transform must survive the rejected insert"
        );
    }

    #[test]
    fn insert_rejects_child_frame_mismatch_dynamic() {
        let mut buffer = Buffer::new();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t3)).unwrap();

        // A different child between the stored samples: without child pinning
        // this insert succeeded and made interpolating lookups fail with
        // IncompatibleFrames while exact-hit lookups kept working.
        let mut other = create_transform(t2);
        other.child = "lidar".into();
        assert!(matches!(
            buffer.insert(other),
            Err(BufferError::ChildFrameMismatch(_))
        ));

        // Interpolation over the pinned child's samples must keep working.
        let result = buffer.get(&t2).unwrap();
        assert_eq!(result.child, "base");
        assert_eq!(result.timestamp, t2);
    }

    #[test]
    fn out_of_order_insert_does_not_regress_latest_timestamp() {
        let mut buffer = Buffer::with_max_age(Duration::from_secs(1));

        let t_new = Timestamp::from_nanos(5_000_000_000);
        let t_old = Timestamp::from_nanos(1_000_000_000);

        buffer.insert(create_transform(t_new)).unwrap();
        // Late-arriving old sample: the expiry reference must remain t_new,
        // so this entry is already outside max_age and gets dropped.
        buffer.insert(create_transform(t_old)).unwrap();

        assert!(
            buffer.get(&t_old).is_err(),
            "expiry must be measured against the latest timestamp, not the last insert"
        );
        assert!(buffer.get(&t_new).is_ok());
    }
}
