use std::time::Duration;
use transforms::{
    errors::TransformError,
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
};
use approx::assert_relative_eq;

#[test]
fn test_transform_creation() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    let transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    assert_relative_eq!(transform.translation.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(transform.translation.y, 2.0, epsilon = f64::EPSILON);
    assert_relative_eq!(transform.translation.z, 3.0, epsilon = f64::EPSILON);
    assert_relative_eq!(transform.rotation.w, 1.0, epsilon = f64::EPSILON);
    assert_eq!(transform.parent, "parent");
    assert_eq!(transform.child, "child");
}

#[test]
fn test_transform_identity() {
    let identity = Transform::identity();
    
    assert_relative_eq!(identity.translation.x, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(identity.translation.y, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(identity.translation.z, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(identity.rotation.w, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(identity.rotation.x, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(identity.rotation.y, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(identity.rotation.z, 0.0, epsilon = f64::EPSILON);
    assert_eq!(identity.timestamp, Timestamp::zero());
    assert_eq!(identity.parent, "");
    assert_eq!(identity.child, "");
}

#[test]
fn test_transform_inverse() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    let transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    let inverse = transform.inverse().unwrap();
    
    // Check that inverse has swapped frames
    assert_eq!(inverse.parent, "child");
    assert_eq!(inverse.child, "parent");
    
    // Check that inverse has negated translation (for identity rotation)
    assert_relative_eq!(inverse.translation.x, -1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.translation.y, -2.0, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.translation.z, -3.0, epsilon = f64::EPSILON);
    
    // Timestamp should remain the same
    assert_eq!(inverse.timestamp, t);
}

#[test]
fn test_transform_inverse_with_rotation() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create a rotation quaternion (90 degrees around Z axis)
    let rot_z_90 = Quaternion {
        w: (std::f64::consts::PI / 4.0).cos(),
        x: 0.0,
        y: 0.0,
        z: (std::f64::consts::PI / 4.0).sin(),
    };

    let transform = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: rot_z_90,
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    let inverse = transform.inverse().unwrap();
    
    // For rotated transforms, the inverse translation is more complex
    // For a 90-degree Z rotation and (1,0,0) translation, the inverse translation should be approximately (0,-1,0)
    assert_relative_eq!(inverse.translation.x, 0.0, epsilon = 0.01);
    assert_relative_eq!(inverse.translation.y, -1.0, epsilon = 0.01);
    assert_relative_eq!(inverse.translation.z, 0.0, epsilon = 0.01);
    
    // The inverse rotation should be the conjugate of the original
    assert_relative_eq!(inverse.rotation.w, transform.rotation.w, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.rotation.x, -transform.rotation.x, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.rotation.y, -transform.rotation.y, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.rotation.z, -transform.rotation.z, epsilon = f64::EPSILON);
}

#[test]
fn test_transform_interpolation() {
    #[cfg(feature = "std")]
    let t_base = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t_base = Timestamp::zero();

    let from = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t_base,
        parent: "parent".into(),
        child: "child".into(),
    };

    let to = Transform {
        translation: Vector3::new(10.0, 20.0, 30.0),
        rotation: Quaternion::identity(),
        timestamp: (t_base + Duration::from_secs(2)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    // Interpolate at the midpoint (t+1s)
    let mid_time = (t_base + Duration::from_secs(1)).unwrap();
    let interpolated = Transform::interpolate(&from, &to, mid_time).unwrap();
    
    assert_relative_eq!(interpolated.translation.x, 5.0, epsilon = f64::EPSILON);
    assert_relative_eq!(interpolated.translation.y, 10.0, epsilon = f64::EPSILON);
    assert_relative_eq!(interpolated.translation.z, 15.0, epsilon = f64::EPSILON);
    assert_eq!(interpolated.timestamp, mid_time);
    assert_eq!(interpolated.parent, "parent");
    assert_eq!(interpolated.child, "child");
}

#[test]
fn test_transform_interpolation_with_rotation() {
    #[cfg(feature = "std")]
    let t_base = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t_base = Timestamp::zero();

    // Starting with identity quaternion
    let from = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t_base,
        parent: "parent".into(),
        child: "child".into(),
    };

    // Ending with 90 degree rotation around Z
    let to = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion {
            w: (std::f64::consts::PI / 4.0).cos(),
            x: 0.0,
            y: 0.0,
            z: (std::f64::consts::PI / 4.0).sin(),
        },
        timestamp: (t_base + Duration::from_secs(2)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    // Interpolate at the midpoint (t+1s) - should be 45 degree rotation
    let mid_time = (t_base + Duration::from_secs(1)).unwrap();
    let interpolated = Transform::interpolate(&from, &to, mid_time).unwrap();
    
    // For 45 degree rotation, w and z should be approximately sqrt(2)/2
    let expected_w_z = (std::f64::consts::PI / 8.0).cos();
    let expected_z = (std::f64::consts::PI / 8.0).sin();
    
    assert_relative_eq!(interpolated.rotation.w, expected_w_z, epsilon = 0.01);
    assert_relative_eq!(interpolated.rotation.x, 0.0, epsilon = 0.01);
    assert_relative_eq!(interpolated.rotation.y, 0.0, epsilon = 0.01);
    assert_relative_eq!(interpolated.rotation.z, expected_z, epsilon = 0.01);
}

#[test]
fn test_transform_interpolation_errors() {
    #[cfg(feature = "std")]
    let t_base = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t_base = Timestamp::zero();

    let from = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t_base,
        parent: "parent1".into(),
        child: "child1".into(),
    };

    let to = Transform {
        translation: Vector3::new(10.0, 20.0, 30.0),
        rotation: Quaternion::identity(),
        timestamp: (t_base + Duration::from_secs(2)).unwrap(),
        parent: "parent2".into(), // Different parent
        child: "child1".into(),
    };

    // Test with incompatible frames
    let mid_time = (t_base + Duration::from_secs(1)).unwrap();
    let result = Transform::interpolate(&from, &to, mid_time);
    assert!(matches!(result, Err(TransformError::IncompatibleFrames)));
    
    // Test with timestamp outside range (too early)
    let to_compatible = Transform {
        translation: Vector3::new(10.0, 20.0, 30.0),
        rotation: Quaternion::identity(),
        timestamp: (t_base + Duration::from_secs(2)).unwrap(),
        parent: "parent1".into(),
        child: "child1".into(),
    };
    
    let early_time = (t_base - Duration::from_secs(1)).unwrap();
    let result = Transform::interpolate(&from, &to_compatible, early_time);
    assert!(matches!(result, Err(TransformError::TimestampMismatch(_, _))));
    
    // Test with timestamp outside range (too late)
    let late_time = (t_base + Duration::from_secs(3)).unwrap();
    let result = Transform::interpolate(&from, &to_compatible, late_time);
    assert!(matches!(result, Err(TransformError::TimestampMismatch(_, _))));
    
    // Test with timestamp out of order (from > to)
    let from_later = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t_base + Duration::from_secs(3)).unwrap(),
        parent: "parent1".into(),
        child: "child1".into(),
    };
    
    let result = Transform::interpolate(&from_later, &to_compatible, mid_time);
    assert!(matches!(result, Err(TransformError::TimestampMismatch(_, _))));
}

#[test]
fn test_transform_multiplication() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // First transform: translate 1 unit in x direction
    let t1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "world".into(),
        child: "a".into(),
    };

    // Second transform: translate 1 unit in y direction
    let t2 = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    // Multiply transforms (t2 * t1)
    let result = (t2 * t1).unwrap();
    
    // Should result in translation of (1,1,0) from world to b
    assert_eq!(result.parent, "world");
    assert_eq!(result.child, "b");
    assert_relative_eq!(result.translation.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(result.translation.y, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(result.translation.z, 0.0, epsilon = f64::EPSILON);
}

#[test]
fn test_transform_multiplication_with_rotation() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create a rotation quaternion (90 degrees around Z axis)
    let rot_z_90 = Quaternion {
        w: (std::f64::consts::PI / 4.0).cos(),
        x: 0.0,
        y: 0.0,
        z: (std::f64::consts::PI / 4.0).sin(),
    };

    // First transform: rotate 90 degrees around Z
    let t1 = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: rot_z_90,
        timestamp: t,
        parent: "world".into(),
        child: "a".into(),
    };

    // Second transform: translate 1 unit in x direction
    let t2 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    // Multiply transforms (t2 * t1)
    let result = (t2 * t1).unwrap();
    
    // The X-axis translation in frame a becomes a Y-axis translation in frame world
    // due to the 90 degree rotation
    assert_eq!(result.parent, "world");
    assert_eq!(result.child, "b");
    assert_relative_eq!(result.translation.x, 0.0, epsilon = 0.01);
    assert_relative_eq!(result.translation.y, 1.0, epsilon = 0.01);
    assert_relative_eq!(result.translation.z, 0.0, epsilon = 0.01);
}

#[test]
fn test_transform_multiplication_errors() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create two transforms
    let t1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "world".into(),
        child: "a".into(),
    };

    // Test timestamps mismatch error
    let t2_different_time = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(10)).unwrap(),
        parent: "a".into(),
        child: "b".into(),
    };
    
    let result = t2_different_time * t1.clone();
    assert!(matches!(result, Err(TransformError::TimestampMismatch(_, _))));
    
    // Test incompatible frames error
    let t2_incompatible = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "c".into(), // Not matching t1's child
        child: "b".into(),
    };
    
    let result = t2_incompatible * t1.clone();
    assert!(matches!(result, Err(TransformError::IncompatibleFrames)));
    
    // Test same frame multiplication error
    let t2_same_frame = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "world".into(),
        child: "a".into(), // Same as t1's child
    };
    
    let result = t2_same_frame * t1;
    assert!(matches!(result, Err(TransformError::SameFrameMultiplication)));
}

#[test]
fn test_transform_equality() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    let t1 = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    let t2 = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    let t3 = Transform {
        translation: Vector3::new(4.0, 5.0, 6.0), // Different translation
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    assert_eq!(t1, t2, "Identical transforms should be equal");
    assert_ne!(t1, t3, "Transforms with different translations should not be equal");
    
    // Check that small differences (within epsilon) are still considered equal
    let t4 = Transform {
        translation: Vector3::new(1.0 + f64::EPSILON/2.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    assert_eq!(t1, t4, "Transforms with very small differences should be equal");
}

#[test]
fn test_transform_edge_cases() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Test with zero translation
    let zero_trans = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let inverse = zero_trans.inverse().unwrap();
    assert_relative_eq!(inverse.translation.x, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.translation.y, 0.0, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse.translation.z, 0.0, epsilon = f64::EPSILON);
    
    // Test with very large translation values
    let large_trans = Transform {
        translation: Vector3::new(1e6, 2e6, 3e6),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let inverse = large_trans.inverse().unwrap();
    assert_relative_eq!(inverse.translation.x, -1e6, epsilon = 0.1);
    assert_relative_eq!(inverse.translation.y, -2e6, epsilon = 0.1);
    assert_relative_eq!(inverse.translation.z, -3e6, epsilon = 0.1);
    
    // Test with non-normalized quaternion
    let non_normalized = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion {
            w: 2.0, 
            x: 0.0, 
            y: 0.0, 
            z: 0.0
        }, // Magnitude 2
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let inverse = non_normalized.inverse().unwrap();
    // The inverse should normalize the quaternion
    assert_relative_eq!(inverse.rotation.w, 1.0, epsilon = f64::EPSILON);
}

#[test]
fn test_special_rotations() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();
    
    // 180 degree rotation around X axis
    let rot_x_180 = Quaternion {
        w: 0.0,
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    
    let transform = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: rot_x_180,
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    // Check that the inverse rotation flips the Y coordinate
    let inverse = transform.inverse().unwrap();
    assert_relative_eq!(inverse.translation.y, 1.0, epsilon = 0.01, "Y coordinate should be flipped by 180° X rotation");
    
    // 180 degree rotation around Y axis
    let rot_y_180 = Quaternion {
        w: 0.0,
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    
    let transform = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: rot_y_180,
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    // Check that the inverse rotation flips the X coordinate
    let inverse = transform.inverse().unwrap();
    assert_relative_eq!(inverse.translation.x, 1.0, epsilon = 0.01, "X coordinate should be flipped by 180° Y rotation");
}