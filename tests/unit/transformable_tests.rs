use std::time::Duration;
use transforms::{
    errors::TransformError,
    geometry::{Point, Quaternion, Transform, Vector3},
    time::Timestamp,
    Transformable,
};
use approx::assert_relative_eq;

#[test]
fn test_point_transform_basic() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create a point in frame "child"
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "child".into(),
    };
    
    // Create a transform from "child" to "parent"
    let transform = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    // Apply transform to point
    let result = point.transform(&transform);
    
    assert!(result.is_ok(), "Transform application failed");
    
    // Check the point is now in the parent frame with coordinates updated
    assert_eq!(point.frame, "parent");
    assert_relative_eq!(point.position.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.y, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.z, 0.0, epsilon = f64::EPSILON);
}

#[test]
fn test_point_transform_with_rotation() {
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

    // Create a point in frame "child"
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "child".into(),
    };
    
    // Create a transform from "child" to "parent" with rotation
    let transform = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0), // No translation, just rotation
        rotation: rot_z_90,
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    // Apply transform to point
    let result = point.transform(&transform);
    
    assert!(result.is_ok(), "Transform application failed");
    
    // Check the point's position is rotated 90 degrees around Z (X becomes Y)
    assert_eq!(point.frame, "parent");
    assert_relative_eq!(point.position.x, 0.0, epsilon = 0.01);
    assert_relative_eq!(point.position.y, 1.0, epsilon = 0.01);
    assert_relative_eq!(point.position.z, 0.0, epsilon = 0.01);
    
    // Check the orientation is also rotated
    assert_relative_eq!(point.orientation.w, rot_z_90.w, epsilon = 0.01);
    assert_relative_eq!(point.orientation.x, rot_z_90.x, epsilon = 0.01);
    assert_relative_eq!(point.orientation.y, rot_z_90.y, epsilon = 0.01);
    assert_relative_eq!(point.orientation.z, rot_z_90.z, epsilon = 0.01);
}

#[test]
fn test_point_transform_errors() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Test frame mismatch error
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "wrong_frame".into(), // Different from transform's child frame
    };
    
    let transform = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let result = point.transform(&transform);
    assert!(matches!(result, Err(TransformError::IncompatibleFrames)));
    
    // Test timestamp mismatch error
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "child".into(),
    };
    
    let different_time = (t + Duration::from_secs(10)).unwrap();
    let transform = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: different_time,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let result = point.transform(&transform);
    assert!(matches!(result, Err(TransformError::TimestampMismatch(_, _))));
}

#[test]
fn test_point_transform_chain() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create a point in frame "child"
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "child".into(),
    };
    
    // Create transform from "child" to "middle"
    let transform1 = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "middle".into(),
        child: "child".into(),
    };
    
    // Create transform from "middle" to "parent"
    let transform2 = Transform {
        translation: Vector3::new(0.0, 0.0, 1.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "middle".into(),
    };
    
    // Apply first transform
    point.transform(&transform1).unwrap();
    
    // Check intermediate state
    assert_eq!(point.frame, "middle");
    assert_relative_eq!(point.position.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.y, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.z, 0.0, epsilon = f64::EPSILON);
    
    // Apply second transform
    point.transform(&transform2).unwrap();
    
    // Check final state
    assert_eq!(point.frame, "parent");
    assert_relative_eq!(point.position.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.y, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.z, 1.0, epsilon = f64::EPSILON);
}

#[test]
fn test_point_transform_with_complex_rotation() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create a rotation around X and then Z axis
    let rot_x_45 = Quaternion {
        w: (std::f64::consts::PI / 8.0).cos(),
        x: (std::f64::consts::PI / 8.0).sin(),
        y: 0.0,
        z: 0.0,
    };
    
    let rot_z_45 = Quaternion {
        w: (std::f64::consts::PI / 8.0).cos(),
        x: 0.0,
        y: 0.0,
        z: (std::f64::consts::PI / 8.0).sin(),
    };
    
    // Combined rotation
    let combined_rot = rot_z_45 * rot_x_45;
    
    // Create a point with non-identity orientation
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: rot_x_45, // Point already has 45° rotation around X
        timestamp: t,
        frame: "child".into(),
    };
    
    // Create a transform with rotation
    let transform = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: rot_z_45, // Transform adds 45° rotation around Z
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    // Apply transform
    point.transform(&transform).unwrap();
    
    // Check that position has been rotated appropriately
    assert_relative_eq!(point.position.x, combined_rot.rotate_vector(Vector3::new(1.0, 0.0, 0.0)).x, epsilon = 0.01);
    assert_relative_eq!(point.position.y, combined_rot.rotate_vector(Vector3::new(1.0, 0.0, 0.0)).y, epsilon = 0.01);
    assert_relative_eq!(point.position.z, combined_rot.rotate_vector(Vector3::new(1.0, 0.0, 0.0)).z, epsilon = 0.01);
    
    // Check that orientation has been combined
    assert_relative_eq!(point.orientation.w, combined_rot.w, epsilon = 0.01);
    assert_relative_eq!(point.orientation.x, combined_rot.x, epsilon = 0.01);
    assert_relative_eq!(point.orientation.y, combined_rot.y, epsilon = 0.01);
    assert_relative_eq!(point.orientation.z, combined_rot.z, epsilon = 0.01);
}

#[test]
fn test_point_transform_edge_cases() {
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Test with identity transform
    let mut point = Point {
        position: Vector3::new(1.0, 2.0, 3.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "frame".into(),
    };
    
    let identity_transform = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "frame".into(),
    };
    
    point.transform(&identity_transform).unwrap();
    
    // Only the frame should change, position and orientation should remain the same
    assert_eq!(point.frame, "parent");
    assert_relative_eq!(point.position.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.y, 2.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.position.z, 3.0, epsilon = f64::EPSILON);
    assert_relative_eq!(point.orientation.w, 1.0, epsilon = f64::EPSILON);
    
    // Test with a 180-degree rotation
    let mut point = Point {
        position: Vector3::new(1.0, 0.0, 0.0),
        orientation: Quaternion::identity(),
        timestamp: t,
        frame: "frame".into(),
    };
    
    // 180-degree rotation around Y-axis
    let rot_y_180 = Quaternion {
        w: 0.0,
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    
    let transform = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: rot_y_180,
        timestamp: t,
        parent: "parent".into(),
        child: "frame".into(),
    };
    
    point.transform(&transform).unwrap();
    
    // Point should be flipped in X direction
    assert_relative_eq!(point.position.x, -1.0, epsilon = 0.01);
    assert_relative_eq!(point.position.y, 0.0, epsilon = 0.01);
    assert_relative_eq!(point.position.z, 0.0, epsilon = 0.01);
}