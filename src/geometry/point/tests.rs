#[cfg(test)]
mod point_tests {
    use crate::{
        Transform, Transformable,
        geometry::{Point, Quaternion, Vector3},
        time::Timestamp,
    };
    use approx::assert_abs_diff_eq;

    #[test]
    fn point_creation() {
        let _ = Point {
            position: Vector3::new(1.0, 2.0, 3.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            frame: "a".into(),
        };
    }

    #[test]
    fn transform_rotates_orientation() {
        let theta = core::f64::consts::PI / 2.0;
        let rot_z_90 = Quaternion::new((theta / 2.0).cos(), 0.0, 0.0, (theta / 2.0).sin());

        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            frame: "b".into(),
        };

        let transform = Transform {
            translation: Vector3::zero(),
            rotation: rot_z_90,
            timestamp: Timestamp::zero(),
            parent: "a".into(),
            child: "b".into(),
        };

        point.transform(&transform).unwrap();

        // The orientation must be rotated (quaternion product), not merely
        // combined component-wise.
        let expected = Point {
            position: Vector3::new(0.0, 1.0, 0.0),
            orientation: rot_z_90,
            timestamp: Timestamp::zero(),
            frame: "a".into(),
        };
        assert_abs_diff_eq!(point, expected, epsilon = 1e-10);
    }

    #[test]
    fn transform_from_a_different_frame_is_rejected_with_both_frames_named() {
        use crate::errors::TransformError;

        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            frame: "lidar".into(),
        };
        // The transform maps "camera" data, not "lidar" data.
        let transform = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::zero(),
            parent: "base".into(),
            child: "camera".into(),
        };

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

        let mut point = Point {
            position: Vector3::new(1.0, 0.0, 0.0),
            orientation: Quaternion::identity(),
            timestamp: Timestamp::from_nanos(1_000_000_000),
            frame: "camera".into(),
        };
        // A dynamic transform from another time must not apply.
        let transform = Transform {
            translation: Vector3::new(0.0, 1.0, 0.0),
            rotation: Quaternion::identity(),
            timestamp: Timestamp::from_nanos(2_000_000_000),
            parent: "base".into(),
            child: "camera".into(),
        };

        match point.transform(&transform) {
            Err(TransformError::TimestampMismatch { lhs, rhs }) => {
                assert_eq!(lhs, 1.0);
                assert_eq!(rhs, 2.0);
            }
            other => panic!("expected TimestampMismatch, got {other:?}"),
        }
    }
}
