use std::time::Duration;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};
use approx::assert_relative_eq;

#[test]
fn test_registry_creation() {
    #[cfg(feature = "std")]
    let _registry = Registry::new(Duration::from_secs(60));

    #[cfg(not(feature = "std"))]
    let _registry = Registry::new();
}

#[test]
fn test_add_and_get_single_transform() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    let transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    registry.add_transform(transform.clone());
    
    let result = registry.get_transform("parent", "child", t);
    assert!(result.is_ok(), "Failed to get transform that was just added");
    
    let retrieved = result.unwrap();
    assert_eq!(retrieved, transform, "Retrieved transform doesn't match the added one");
}

#[test]
fn test_transform_interpolation() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Add two transforms a second apart
    let t1 = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };

    let t2 = Transform {
        translation: Vector3::new(10.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(1)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    registry.add_transform(t1);
    registry.add_transform(t2);
    
    // Get transform at midpoint (500ms later)
    let mid_time = (t + Duration::from_millis(500)).unwrap();
    let result = registry.get_transform("parent", "child", mid_time);
    
    assert!(result.is_ok(), "Failed to get interpolated transform");
    
    let interpolated = result.unwrap();
    assert_relative_eq!(interpolated.translation.x, 5.0, epsilon = f64::EPSILON);
}

#[test]
fn test_transform_chain() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Set up a chain: world -> robot -> arm -> gripper
    let t_world_robot = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "world".into(),
        child: "robot".into(),
    };

    let t_robot_arm = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "robot".into(),
        child: "arm".into(),
    };

    let t_arm_gripper = Transform {
        translation: Vector3::new(0.0, 0.0, 1.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "arm".into(),
        child: "gripper".into(),
    };

    registry.add_transform(t_world_robot);
    registry.add_transform(t_robot_arm);
    registry.add_transform(t_arm_gripper);
    
    // Try to get transform from world to gripper (should chain all transforms)
    let result = registry.get_transform("world", "gripper", t);
    
    assert!(result.is_ok(), "Failed to get chained transform");
    
    let chained = result.unwrap();
    assert_relative_eq!(chained.translation.x, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(chained.translation.y, 1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(chained.translation.z, 1.0, epsilon = f64::EPSILON);
}

#[test]
fn test_nonexistent_transform() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Try to get a transform that doesn't exist
    let result = registry.get_transform("nonexistent_parent", "nonexistent_child", t);
    
    assert!(result.is_err(), "Expected error when getting nonexistent transform");
}

#[test]
fn test_delete_transforms_before() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t_base = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t_base = Timestamp::zero();

    // Add transforms at different times
    let t1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t_base,
        parent: "parent".into(),
        child: "child".into(),
    };

    let t2 = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t_base + Duration::from_secs(1)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    let t3 = Transform {
        translation: Vector3::new(3.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t_base + Duration::from_secs(2)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    registry.add_transform(t1);
    registry.add_transform(t2.clone());
    registry.add_transform(t3.clone());
    
    // Delete transforms before the middle timestamp
    let delete_time = (t_base + Duration::from_secs(1)).unwrap();
    registry.delete_transforms_before(delete_time);
    
    // The first transform should be gone, but the other two should remain
    let result1 = registry.get_transform("parent", "child", t_base);
    let result2 = registry.get_transform("parent", "child", (t_base + Duration::from_secs(1)).unwrap());
    let result3 = registry.get_transform("parent", "child", (t_base + Duration::from_secs(2)).unwrap());
    
    assert!(result1.is_err(), "Transform before delete threshold should be gone");
    assert!(result2.is_ok(), "Transform at delete threshold should exist");
    assert!(result3.is_ok(), "Transform after delete threshold should exist");
    
    assert_eq!(result2.unwrap(), t2, "Retrieved transform doesn't match expected");
    assert_eq!(result3.unwrap(), t3, "Retrieved transform doesn't match expected");
}

#[test]
fn test_inverse_transformation_chain() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Set up a chain: A -> B -> C
    let t_a_b = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    let t_b_c = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b);
    registry.add_transform(t_b_c);
    
    // Get transform from C to A (reverse direction)
    let result = registry.get_transform("c", "a", t);
    
    assert!(result.is_ok(), "Failed to get inverse chained transform");
    
    let inverse_chain = result.unwrap();
    assert_relative_eq!(inverse_chain.translation.x, -1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse_chain.translation.y, -1.0, epsilon = f64::EPSILON);
    assert_relative_eq!(inverse_chain.translation.z, 0.0, epsilon = f64::EPSILON);
}

#[test]
fn test_rotated_transform_chain() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create a rotation quaternion (90 degrees around Z axis)
    let rot_z_90 = Quaternion {
        w: (std::f64::consts::PI / 4.0).cos(),
        x: 0.0,
        y: 0.0,
        z: (std::f64::consts::PI / 4.0).sin(),
    };

    // Set up a chain with rotation: A -> B -> C
    let t_a_b = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: rot_z_90,
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    let t_b_c = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b);
    registry.add_transform(t_b_c);
    
    // Get transform from A to C
    let result = registry.get_transform("a", "c", t);
    
    assert!(result.is_ok(), "Failed to get rotated transform chain");
    
    let chain = result.unwrap();
    // After rotation, the B->C translation of (1,0,0) becomes approximately (0,1,0) in A's frame
    assert_relative_eq!(chain.translation.x, 1.0, epsilon = 0.01);
    assert_relative_eq!(chain.translation.y, 1.0, epsilon = 0.01);
}

#[test]
fn test_timestamp_not_in_range() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Add two transforms with timestamps 1 sec and 3 sec from base
    let t1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(1)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    let t2 = Transform {
        translation: Vector3::new(3.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: (t + Duration::from_secs(3)).unwrap(),
        parent: "parent".into(),
        child: "child".into(),
    };

    registry.add_transform(t1);
    registry.add_transform(t2);
    
    // Try to get transform at t = base (too early)
    let result_early = registry.get_transform("parent", "child", t);
    assert!(result_early.is_err(), "Getting transform before earliest timestamp should fail");
    
    // Try to get transform at t = base + 4s (too late)
    let result_late = registry.get_transform("parent", "child", (t + Duration::from_secs(4)).unwrap());
    assert!(result_late.is_err(), "Getting transform after latest timestamp should fail");
    
    // But getting a transform in the middle should work (interpolation)
    let result_mid = registry.get_transform("parent", "child", (t + Duration::from_secs(2)).unwrap());
    assert!(result_mid.is_ok(), "Getting transform within timestamp range should succeed");
}

#[test]
fn test_disconnected_transform_tree() {
    let _ = env_logger::try_init();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create two disconnected transform chains: A->B->C and D->E->F
    let t_a_b = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    let t_b_c = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "b".into(),
        child: "c".into(),
    };

    let t_d_e = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "d".into(),
        child: "e".into(),
    };

    let t_e_f = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "e".into(),
        child: "f".into(),
    };

    registry.add_transform(t_a_b);
    registry.add_transform(t_b_c);
    registry.add_transform(t_d_e);
    registry.add_transform(t_e_f);
    
    // Try to get transform between disconnected parts of the tree
    let result = registry.get_transform("a", "f", t);
    
    assert!(result.is_err(), "Getting transform between disconnected tree parts should fail");
}