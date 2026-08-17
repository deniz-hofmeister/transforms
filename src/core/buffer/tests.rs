#[cfg(test)]
mod buffer_tests {
    use crate::{
        core::{Buffer, buffer::BufferError},
        errors::TransformError,
        geometry::{Quaternion, Transform, Vector3},
        time::{Stamp, Timestamp},
    };
    use core::time::Duration;

    fn create_transform(t: Timestamp) -> Transform {
        stamped_transform(Stamp::At(t))
    }

    fn create_static_transform() -> Transform {
        stamped_transform(Stamp::Static)
    }

    fn stamped_transform(timestamp: Stamp) -> Transform {
        frames_transform("map", "base", Vector3::new(1.0, 2.0, 3.0), timestamp)
    }

    /// A valid transform between the named frames, for the tests that vary
    /// frames or translation. `Transform`'s fields are private, so a variant
    /// is built, not edited into existence.
    fn frames_transform(
        parent: &str,
        child: &str,
        translation: Vector3,
        timestamp: Stamp,
    ) -> Transform {
        Transform::new(
            parent,
            child,
            translation,
            Quaternion::identity(),
            timestamp,
        )
        .unwrap()
    }

    #[test]
    fn insert_and_get() {
        let mut buffer = Buffer::dynamic();
        let t = Timestamp::from_nanos(1_000_000_000);

        let transform = create_transform(t);
        buffer.insert(transform.clone()).unwrap();

        let mut r = buffer.get(t);

        assert!(r.is_ok(), "expected transform, got {r:?}");
        assert_eq!(r.unwrap(), transform);

        r = buffer.get((t + Duration::from_secs(1)).unwrap());
        assert!(r.is_err(), "expected no transform, got {r:?}");

        r = buffer.get((t - Duration::from_secs(1)).unwrap());
        assert!(r.is_err(), "expected no transform, got {r:?}");
    }

    #[test]
    // The compared seconds are exactly representable; the assertion is on
    // the reported values, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn get_out_of_range_reports_covered_range() {
        let mut buffer = Buffer::dynamic();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t2)).unwrap();

        // Too new: past the latest sample. The error carries the requested
        // time and the covered range, so latency ("just too new") is
        // distinguishable from stale data without further queries.
        let result = buffer.get(Timestamp::from_nanos(3_000_000_000));
        assert!(
            matches!(
                &result,
                Err(BufferError::TransformError(
                    TransformError::TimestampOutOfRange { requested, start, end }
                )) if *requested == 3.0 && *start == 1.0 && *end == 2.0
            ),
            "expected TimestampOutOfRange with the covered range, got {result:?}"
        );

        // Too old: before the earliest sample.
        let result = buffer.get(Timestamp::from_nanos(500_000_000));
        assert!(
            matches!(
                &result,
                Err(BufferError::TransformError(
                    TransformError::TimestampOutOfRange { requested, start, end }
                )) if *requested == 0.5 && *start == 1.0 && *end == 2.0
            ),
            "expected TimestampOutOfRange with the covered range, got {result:?}"
        );
    }

    #[test]
    fn get_on_empty_buffer_reports_no_transforms() {
        let buffer = Buffer::<Timestamp>::dynamic();

        let result = buffer.get(Timestamp::from_nanos(1_000_000_000));
        assert!(
            matches!(result, Err(BufferError::NoTransformAvailable)),
            "expected NoTransformAvailable on an empty dynamic buffer, got {result:?}"
        );

        let buffer = Buffer::<Timestamp>::static_edge();
        let result = buffer.get(Timestamp::from_nanos(1_000_000_000));
        assert!(
            matches!(result, Err(BufferError::NoTransformAvailable)),
            "expected NoTransformAvailable on an empty static buffer, got {result:?}"
        );

        let buffer = Buffer::<Timestamp>::dynamic_with_max_age(Duration::from_secs(10));
        let result = buffer.get(Timestamp::from_nanos(1_000_000_000));
        assert!(
            matches!(result, Err(BufferError::NoTransformAvailable)),
            "expected NoTransformAvailable on an empty max-age buffer, got {result:?}"
        );
    }

    #[test]
    fn insert_and_get_static() {
        let mut buffer = Buffer::static_edge();

        let transform = create_static_transform();

        buffer.insert(transform.clone()).unwrap();

        // A static buffer serves any requested instant.
        let mut r = buffer.get(Timestamp::from_nanos(1_000_000_000));

        assert!(r.is_ok(), "expected transform, got {r:?}");
        assert_eq!(r.unwrap(), transform);

        r = buffer.get(Timestamp::zero());
        assert!(r.is_ok(), "expected transform, got {r:?}");
        assert_eq!(r.unwrap(), transform);
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn static_insert_is_a_last_write_wins_upsert() {
        let mut buffer = Buffer::static_edge();

        buffer.insert(create_static_transform()).unwrap();

        let recalibrated =
            frames_transform("map", "base", Vector3::new(9.0, 9.0, 9.0), Stamp::Static);
        buffer.insert(recalibrated).unwrap();

        assert_eq!(buffer.get(Timestamp::zero()).unwrap().translation().x, 9.0);
        // The replacement is served at every instant, so the original is
        // stored nowhere: a static buffer holds one transform, not a history.
        assert_eq!(
            buffer
                .get(Timestamp::from_nanos(9_000_000_000))
                .unwrap()
                .translation()
                .x,
            9.0
        );
    }

    #[test]
    fn zero_is_an_ordinary_dynamic_timestamp() {
        // The most natural loop there is: samples at t = 0, 1, 2, ... The
        // old design reserved t = 0 as the static sentinel, so the very
        // first iteration silently created a static buffer and the second
        // failed with StaticDynamicConflict. With staticness in the Stamp,
        // zero needs no special handling.
        let mut buffer = Buffer::dynamic();
        for i in 0..3u64 {
            let t = Timestamp::from_nanos(i * 1_000_000_000);
            buffer.insert(create_transform(t)).unwrap();
        }

        // All three samples are stored and served exactly, the t = 0 one
        // included.
        for i in 0..3u64 {
            let t = Timestamp::from_nanos(i * 1_000_000_000);
            assert_eq!(buffer.get(t).unwrap().timestamp(), Stamp::At(t));
        }

        // Interpolation across t = 0 works like any other span.
        let midpoint = Timestamp::from_nanos(500_000_000);
        let interpolated = buffer.get(midpoint).unwrap();
        assert_eq!(interpolated.timestamp(), Stamp::At(midpoint));
    }

    #[test]
    fn remove_before_resets_the_expiry_reference() {
        // Regression test: remove_before used to clear only the sample map,
        // leaving the max_age expiry reference at the pre-wipe maximum. A
        // restarted stream at earlier timestamps was then evicted by the
        // very insert that added it — Ok(()) returned, buffer stayed empty.
        let mut buffer = Buffer::dynamic_with_max_age(Duration::from_secs(1));
        buffer
            .insert(create_transform(Timestamp::from_nanos(1_000_000_000_000)))
            .unwrap();

        buffer.remove_before(Timestamp::from_nanos(2_000_000_000_000));
        assert!(matches!(
            buffer.get(Timestamp::from_nanos(1_000_000_000_000)),
            Err(BufferError::NoTransformAvailable)
        ));

        // A restarted stream from t = 0 must be retained again.
        let t = Timestamp::zero();
        buffer.insert(create_transform(t)).unwrap();
        assert!(buffer.get(t).is_ok());
    }

    #[test]
    fn emptied_dynamic_buffer_keeps_its_kind() {
        // Regression test: the kind used to be a flag re-decided whenever
        // the buffer was empty, so a dynamic buffer emptied by
        // remove_before accepted a static insert, silently flipped kind,
        // and then rejected dynamic inserts. The kind is now declared at
        // construction and structural: it cannot flip.
        let t = Timestamp::from_nanos(1_000_000_000);
        let mut buffer = Buffer::dynamic();
        buffer.insert(create_transform(t)).unwrap();

        buffer.remove_before((t + Duration::from_secs(1)).unwrap());
        assert!(matches!(
            buffer.get(t),
            Err(BufferError::NoTransformAvailable)
        ));

        let result = buffer.insert(create_static_transform());
        assert!(
            matches!(result, Err(BufferError::StaticDynamicConflict)),
            "an emptied dynamic buffer must stay dynamic, got {result:?}"
        );

        // Dynamic inserts keep working.
        buffer.insert(create_transform(t)).unwrap();
        assert!(buffer.get(t).is_ok());
    }

    #[test]
    fn get_nearest() {
        let mut buffer = Buffer::dynamic();
        let t = Timestamp::from_nanos(1_000_000_000);

        let t1 = (t + Duration::from_secs(1)).unwrap();
        let t2 = (t + Duration::from_secs(2)).unwrap();
        let t3 = (t + Duration::from_secs(3)).unwrap();
        let p1 = create_transform(t1);
        let p2 = create_transform(t2);
        let p3 = create_transform(t3);

        buffer.insert(p1.clone()).unwrap();
        buffer.insert(p2.clone()).unwrap();
        buffer.insert(p3.clone()).unwrap();

        // Exact match
        let (before, after) = buffer.get_nearest(&t2);
        assert_eq!(before.unwrap(), (&t2, &p2));
        assert_eq!(after.unwrap(), (&t2, &p2));

        // Between two points
        let p_mid = (t1 + Duration::from_millis(500)).unwrap();
        let (before, after) = buffer.get_nearest(&p_mid);
        assert_eq!(before.unwrap(), (&t1, &p1));
        assert_eq!(after.unwrap(), (&t2, &p2));

        // Before first point
        let p_0 = (t1 - Duration::from_secs(1)).unwrap();
        let (before, after) = buffer.get_nearest(&p_0);
        assert_eq!(before, None);
        assert_eq!(after.unwrap(), (&t1, &p1));

        // After last point
        let p_4 = (t3 + Duration::from_secs(1)).unwrap();
        let (before, after) = buffer.get_nearest(&p_4);
        assert_eq!(before.unwrap(), (&t3, &p3));
        assert_eq!(after, None);

        // Exactly at first point
        let (before, after) = buffer.get_nearest(&t1);
        assert_eq!(before.unwrap(), (&t1, &p1));
        assert_eq!(after.unwrap(), (&t1, &p1));

        // Exactly at last point
        let (before, after) = buffer.get_nearest(&t3);
        assert_eq!(before.unwrap(), (&t3, &p3));
        assert_eq!(after.unwrap(), (&t3, &p3));
    }

    #[test]
    fn empty_buffer() {
        let buffer = Buffer::dynamic();

        assert!(buffer.get(Timestamp::from_nanos(1000)).is_err());

        let (before, after) = buffer.get_nearest(&Timestamp::from_nanos(1000));
        assert!(before.is_none());
        assert!(after.is_none());
    }

    #[test]
    fn remove_before() {
        let mut buffer = Buffer::dynamic();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = (t1 + Duration::from_secs(2)).unwrap();

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t2)).unwrap();

        assert!(buffer.get(t1).is_ok());
        assert!(buffer.get(t2).is_ok());

        buffer.remove_before(Timestamp::from_nanos(2_000_000_000));

        assert!(buffer.get(t1).is_err());
        assert!(buffer.get(t2).is_ok());
    }

    #[test]
    fn remove_expired() {
        let mut buffer = Buffer::dynamic_with_max_age(Duration::from_secs(10));
        let t = Timestamp::from_nanos(20_000_000_000);

        buffer.insert(create_transform(t)).unwrap();
        buffer
            .insert(create_transform((t + Duration::from_secs(1)).unwrap()))
            .unwrap();
        buffer
            .insert(create_transform((t + Duration::from_secs(2)).unwrap()))
            .unwrap();

        let get_1 = buffer.get((t - Duration::from_secs(2)).unwrap());
        let get_2 = buffer.get((t - Duration::from_secs(1)).unwrap());
        let get_3 = buffer.get(t);

        assert!(get_1.is_err());
        // Before the earliest stored sample: nothing to interpolate from.
        assert!(get_2.is_err());
        assert!(get_3.is_ok());
    }

    #[test]
    fn single_point_buffer() {
        let mut buffer = Buffer::dynamic();
        let t = Timestamp::from_nanos(1_000_000_000);

        let point = create_transform(t);
        buffer.insert(point.clone()).unwrap();

        // Before the point
        let (before, after) = buffer.get_nearest(&(t - Duration::from_secs(1)).unwrap());
        assert!(before.is_none());
        assert_eq!(after.unwrap(), (&t, &point));

        // Exact match
        let (before, after) = buffer.get_nearest(&t);
        assert_eq!(before.unwrap(), (&t, &point));
        assert_eq!(after.unwrap(), (&t, &point));

        // After the point
        let (before, after) = buffer.get_nearest(&(t + Duration::from_secs(1)).unwrap());
        assert_eq!(before.unwrap(), (&t, &point));
        assert!(after.is_none());
    }

    #[test]
    fn insert_rejects_static_dynamic_mixing() {
        let t_dynamic = Timestamp::from_nanos(1_000_000_000);

        let static_tf = create_static_transform();
        let dynamic_tf = create_transform(t_dynamic);

        // A static buffer rejects dynamic transforms.
        let mut buffer = Buffer::static_edge();

        buffer.insert(static_tf.clone()).unwrap();
        assert!(matches!(
            buffer.insert(dynamic_tf.clone()),
            Err(BufferError::StaticDynamicConflict)
        ));

        // The static transform is still served after the rejected insert.
        assert_eq!(buffer.get(t_dynamic).unwrap(), static_tf);

        // A dynamic buffer rejects static transforms.
        let mut buffer = Buffer::dynamic();

        buffer.insert(dynamic_tf.clone()).unwrap();
        assert!(matches!(
            buffer.insert(static_tf),
            Err(BufferError::StaticDynamicConflict)
        ));

        // The dynamic transform is still served after the rejected insert.
        assert_eq!(buffer.get(t_dynamic).unwrap(), dynamic_tf);
    }

    #[test]
    fn insert_kind_mismatch_on_fresh_buffer_pins_no_frames() {
        // A first insert rejected for the wrong kind must leave the buffer
        // untouched: no frames pinned, nothing stored.
        let mut buffer = Buffer::static_edge();
        let result = buffer.insert(create_transform(Timestamp::zero()));
        assert!(matches!(result, Err(BufferError::StaticDynamicConflict)));
        assert_eq!(buffer.parent(), None);
        assert!(matches!(
            buffer.get(Timestamp::zero()),
            Err(BufferError::NoTransformAvailable)
        ));

        // Neither frame was pinned: a static transform for an entirely
        // different frame pair is still accepted.
        let other = frames_transform("odom", "lidar", Vector3::zero(), Stamp::Static);
        buffer.insert(other).unwrap();
        assert_eq!(buffer.parent(), Some("odom"));
    }

    #[test]
    fn insert_expires_entries_older_than_max_age() {
        let mut buffer = Buffer::dynamic_with_max_age(Duration::from_secs(1));

        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(6_000_000_000);

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t2)).unwrap();

        // t1 is more than max_age older than the latest inserted timestamp,
        // so it must have been expired by the second insert.
        assert!(
            buffer.get(t1).is_err(),
            "entry older than max_age must expire on insert"
        );
        assert!(buffer.get(t2).is_ok());
    }

    #[test]
    fn dynamic_buffer_never_expires_entries() {
        let mut buffer: Buffer = Buffer::dynamic();

        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(3_600_000_000_000);

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t2)).unwrap();

        assert!(
            buffer.get(t1).is_ok(),
            "Buffer::dynamic must not expire entries"
        );
        assert!(buffer.get(t2).is_ok());
    }

    #[test]
    fn remove_before_preserves_static_transforms() {
        let mut buffer: Buffer = Buffer::static_edge();

        let static_tf = create_static_transform();
        buffer.insert(static_tf.clone()).unwrap();

        // Manual cleanup with any cutoff must not destroy a static transform:
        // it is valid for all time, not just before the cutoff.
        buffer.remove_before(Timestamp::from_nanos(5_000_000_000));

        assert_eq!(
            buffer.get(Timestamp::from_nanos(9_000_000_000)).unwrap(),
            static_tf,
            "static transforms must survive manual cleanup"
        );
    }

    #[test]
    fn frame_pins_survive_the_buffer_being_emptied() {
        let mut buffer = Buffer::dynamic();
        assert_eq!(buffer.parent(), None);

        let t = Timestamp::from_nanos(1_000_000_000);
        buffer.insert(create_transform(t)).unwrap();
        assert_eq!(buffer.parent(), Some("map"));

        // The pins survive the buffer being emptied, matching the documented
        // parent behavior: dropping the buffer is the only release.
        buffer.remove_before((t + Duration::from_secs(1)).unwrap());
        assert!(matches!(
            buffer.get(t),
            Err(BufferError::NoTransformAvailable)
        ));
        assert_eq!(buffer.parent(), Some("map"));

        // The child pin survives too, so a drained buffer still refuses a
        // transform for another child frame.
        let other = frames_transform("map", "lidar", Vector3::zero(), Stamp::At(t));
        let result = buffer.insert(other);
        assert!(
            matches!(result, Err(BufferError::ChildFrameMismatch(ref pinned)) if pinned == "base"),
            "expected ChildFrameMismatch, got {result:?}"
        );
    }

    #[test]
    fn insert_rejects_child_frame_mismatch_static() {
        let mut buffer = Buffer::static_edge();

        // Static calibration transform for map -> base.
        let original = create_static_transform();
        buffer.insert(original.clone()).unwrap();

        // Same parent, different child (a frame-naming bug): without child
        // pinning this key collision silently overwrote the stored data.
        let other = frames_transform("map", "lidar", Vector3::new(9.0, 9.0, 9.0), Stamp::Static);
        let result = buffer.insert(other);
        assert!(
            matches!(result, Err(BufferError::ChildFrameMismatch(ref pinned)) if pinned == "base"),
            "expected ChildFrameMismatch, got {result:?}"
        );

        // The original static transform must be untouched and retrievable.
        assert_eq!(
            buffer.get(Timestamp::from_nanos(1_000_000_000)).unwrap(),
            original,
            "the pinned child's static transform must survive the rejected insert"
        );
    }

    #[test]
    fn insert_rejects_child_frame_mismatch_dynamic() {
        let mut buffer = Buffer::dynamic();
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let t3 = Timestamp::from_nanos(3_000_000_000);

        buffer.insert(create_transform(t1)).unwrap();
        buffer.insert(create_transform(t3)).unwrap();

        // A different child between the stored samples: without child pinning
        // this insert succeeded and made interpolating lookups fail with
        // IncompatibleFrames while exact-hit lookups kept working.
        let other = frames_transform("map", "lidar", Vector3::zero(), Stamp::At(t2));
        assert!(matches!(
            buffer.insert(other),
            Err(BufferError::ChildFrameMismatch(_))
        ));

        // Interpolation over the pinned child's samples must keep working.
        let result = buffer.get(t2).unwrap();
        assert_eq!(result.child(), "base");
        assert_eq!(result.timestamp(), Stamp::At(t2));
    }

    #[test]
    fn out_of_order_insert_does_not_regress_latest_timestamp() {
        let mut buffer = Buffer::dynamic_with_max_age(Duration::from_secs(1));

        let t_new = Timestamp::from_nanos(5_000_000_000);
        let t_old = Timestamp::from_nanos(1_000_000_000);

        buffer.insert(create_transform(t_new)).unwrap();
        // Late-arriving old sample: the expiry reference must remain t_new,
        // so this entry is already outside max_age and gets dropped.
        buffer.insert(create_transform(t_old)).unwrap();

        assert!(
            buffer.get(t_old).is_err(),
            "expiry must be measured against the latest timestamp, not the last insert"
        );
        assert!(buffer.get(t_new).is_ok());
    }

    /// A transform translated by `x`, distinguishable from `create_transform`.
    fn transform_with_x(
        t: Timestamp,
        x: f64,
    ) -> Transform {
        frames_transform("a", "b", Vector3::new(x, 0.0, 0.0), Stamp::At(t))
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn duplicate_timestamp_insert_is_a_last_write_wins_upsert() {
        let t = Timestamp::from_nanos(5_000_000_000);
        let mut buffer = Buffer::dynamic();
        buffer.insert(transform_with_x(t, 1.0)).unwrap();
        // Same timestamp, different payload: Ok, silently replaces.
        buffer.insert(transform_with_x(t, 2.0)).unwrap();
        assert_eq!(buffer.get(t).unwrap().translation().x, 2.0);
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn zero_max_age_out_of_order_insert_is_ok_but_immediately_evicted() {
        // Inserting an OLDER sample after a newer one with max_age == ZERO
        // returns Ok, but the same insert call evicts it: the expiry
        // threshold stays pinned to the latest timestamp seen.
        let t1 = Timestamp::from_nanos(1_000_000_000);
        let t2 = Timestamp::from_nanos(2_000_000_000);
        let mut buffer = Buffer::dynamic_with_max_age(Duration::ZERO);
        buffer.insert(transform_with_x(t2, 2.0)).unwrap();
        buffer.insert(transform_with_x(t1, 1.0)).unwrap();
        assert_eq!(buffer.get(t2).unwrap().translation().x, 2.0);
        assert!(buffer.get(t1).is_err(), "older insert must be evicted");
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn eviction_boundary_retains_a_sample_exactly_max_age_old() {
        let max_age = Duration::from_secs(10);
        let t0 = Timestamp::from_nanos(100_000_000_000);
        let t0_plus_max_age = Timestamp::from_nanos(110_000_000_000);

        let mut buffer = Buffer::dynamic_with_max_age(max_age);
        buffer.insert(transform_with_x(t0, 1.0)).unwrap();
        buffer
            .insert(transform_with_x(t0_plus_max_age, 2.0))
            .unwrap();

        // threshold = (t0 + max_age) - max_age = t0; eviction keeps
        // k >= threshold, so the sample exactly max_age old survives.
        assert_eq!(buffer.get(t0).unwrap().translation().x, 1.0);
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn eviction_boundary_evicts_one_nanosecond_past_max_age() {
        let max_age = Duration::from_secs(10);
        let t0 = Timestamp::from_nanos(100_000_000_000);
        let t_past = Timestamp::from_nanos(110_000_000_001);

        let mut buffer = Buffer::dynamic_with_max_age(max_age);
        buffer.insert(transform_with_x(t0, 1.0)).unwrap();
        buffer.insert(transform_with_x(t_past, 2.0)).unwrap();

        assert!(
            buffer.get(t0).is_err(),
            "sample older than max_age must be evicted"
        );
        assert_eq!(buffer.get(t_past).unwrap().translation().x, 2.0);
    }
}
