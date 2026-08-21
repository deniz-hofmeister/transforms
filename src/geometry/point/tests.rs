#[cfg(test)]
mod point_tests {
    use crate::{
        Transform, Transformable,
        geometry::{Point, Quaternion, Vector3},
        time::{Stamp, Timestamp},
    };
    use approx::assert_abs_diff_eq;

    #[test]
    fn new_exposes_the_components_it_was_built_from() {
        let point = Point::new(
            Vector3::new(1.0, 2.0, 3.0),
            Quaternion::identity(),
            Timestamp::zero(),
            "a",
        );

        assert_eq!(point.position, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(point.orientation, Quaternion::identity());
        assert_eq!(point.timestamp, Timestamp::zero());
        assert_eq!(point.frame, "a");
    }

    #[test]
    fn transform_rotates_orientation() {
        let theta = core::f64::consts::PI / 2.0;
        let rot_z_90 = Quaternion::from_wxyz((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin());

        let mut point = Point::new(
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Timestamp::zero(),
            "b",
        );

        let transform = Transform::new(
            "a",
            "b",
            Vector3::zero(),
            rot_z_90,
            Stamp::At(Timestamp::zero()),
        )
        .unwrap();

        point.transform(&transform).unwrap();

        // The orientation must be rotated (quaternion product), not merely
        // combined component-wise.
        let expected = Point::new(
            Vector3::new(0.0, 1.0, 0.0),
            rot_z_90,
            Timestamp::zero(),
            "a",
        );
        assert_abs_diff_eq!(point, expected, epsilon = 1e-10);
    }

    #[test]
    fn transform_from_a_different_frame_is_rejected_with_both_frames_named() {
        use crate::errors::TransformError;

        let mut point = Point::new(
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Timestamp::zero(),
            "lidar",
        );
        // The transform maps "camera" data, not "lidar" data.
        let transform = Transform::new(
            "base",
            "camera",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(Timestamp::zero()),
        )
        .unwrap();

        match point.transform(&transform) {
            Err(TransformError::IncompatibleFrames { expected, found }) => {
                assert_eq!(expected, "camera");
                assert_eq!(found, "lidar");
            }
            other => panic!("expected IncompatibleFrames, got {other:?}"),
        }
    }

    #[test]
    // The compared values are exactly representable; the assertion is on
    // reported payloads, not on float arithmetic.
    #[allow(clippy::float_cmp)]
    fn transform_from_a_different_time_is_rejected_with_both_times_named() {
        use crate::errors::TransformError;

        let mut point = Point::new(
            Vector3::new(1.0, 0.0, 0.0),
            Quaternion::identity(),
            Timestamp::from_nanos(1_000_000_000),
            "camera",
        );
        // A dynamic transform from another time must not apply.
        let transform = Transform::new(
            "base",
            "camera",
            Vector3::new(0.0, 1.0, 0.0),
            Quaternion::identity(),
            Stamp::At(Timestamp::from_nanos(2_000_000_000)),
        )
        .unwrap();

        match point.transform(&transform) {
            Err(TransformError::TimestampMismatch { lhs, rhs }) => {
                assert_eq!(lhs, 1.0);
                assert_eq!(rhs, 2.0);
            }
            other => panic!("expected TimestampMismatch, got {other:?}"),
        }
    }
}
