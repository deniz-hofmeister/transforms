// Main integration tests
use log::debug;
use std::time::Duration;
use transforms::{
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
    Registry,
};

// Import the unit test modules
#[path = "unit/registry_tests.rs"]
mod registry_tests;

#[path = "unit/transform_tests.rs"]
mod transform_tests;

#[path = "unit/buffer_tests.rs"]
mod buffer_tests;

#[path = "unit/timestamp_tests.rs"]
mod timestamp_tests;

#[path = "unit/transformable_tests.rs"]
mod transformable_tests;

// Original tests
#[test]
fn test_matching_tree() {
    let _ = env_logger::try_init();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    // Child frame B at t=0, x=1m without rotation
    let t_a_b_0 = Transform {
        translation: Vector3 {
            x: 1.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    // Child frame B at t=1, x=2m without rotation
    let t_a_b_1 = Transform {
        translation: Vector3 {
            x: 2.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: (t + Duration::from_millis(1000)).unwrap(),
        parent: "a".into(),
        child: "b".into(),
    };
    // Child frame C at t=0, y=1m without rotation
    let t_b_c_0 = Transform {
        translation: Vector3 {
            x: 0.,
            y: 1.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: (t + Duration::from_millis(500)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    // Child frame B at t=1, y=2m without rotation
    let t_b_c_1 = Transform {
        translation: Vector3 {
            x: 0.,
            y: 2.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: (t + Duration::from_millis(1500)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b_0.clone());
    registry.add_transform(t_a_b_1.clone());
    registry.add_transform(t_b_c_0.clone());
    registry.add_transform(t_b_c_1.clone());

    let middle_timestamp = (t + Duration::from_millis(750)).unwrap();
    let t_a_c = Transform {
        translation: Vector3 {
            x: 1.75,
            y: 1.25,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: middle_timestamp,
        parent: "a".into(),
        child: "c".into(),
    };

    let r = registry.get_transform("a", "c", middle_timestamp);

    debug!("Result: {:?}", r);
    debug!("Expected: {:?}", t_a_c);

    assert!(r.is_ok(), "Registry returned Error, expected Ok");
    assert_eq!(
        r.unwrap(),
        t_a_c,
        "Registry returned a transform that is different"
    );
}

#[test]
fn test_non_matching_tree() {
    let _ = env_logger::try_init();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    // Child frame B at t=0, x=1m without rotation
    let t_a_b_0 = Transform {
        translation: Vector3 {
            x: 1.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: t,
        parent: "a".into(),
        child: "b".into(),
    };

    // Child frame B at t=1, x=2m without rotation
    let t_a_b_1 = Transform {
        translation: Vector3 {
            x: 2.,
            y: 0.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: (t + Duration::from_secs(1)).unwrap(),
        parent: "a".into(),
        child: "b".into(),
    };

    // Child frame C at t=0, y=1m without rotation
    let t_b_c_0 = Transform {
        translation: Vector3 {
            x: 0.,
            y: 1.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: (t + Duration::from_secs(2)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    // Child frame B at t=1, y=2m without rotation
    let t_b_c_1 = Transform {
        translation: Vector3 {
            x: 0.,
            y: 2.,
            z: 0.,
        },
        rotation: Quaternion {
            w: 1.,
            x: 0.,
            y: 0.,
            z: 0.,
        },
        timestamp: (t + Duration::from_secs(3)).unwrap(),
        parent: "b".into(),
        child: "c".into(),
    };

    registry.add_transform(t_a_b_0.clone());
    registry.add_transform(t_a_b_1.clone());
    registry.add_transform(t_b_c_0.clone());
    registry.add_transform(t_b_c_1.clone());

    let r = registry.get_transform("a", "c", t);

    debug!("Result: {:?}", r);

    assert!(r.is_err(), "Registry returned Ok, expected Err");
}

// Add more end-to-end integration tests for complex scenarios

#[test]
fn test_complex_transform_chain() {
    let _ = env_logger::try_init();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();

    // Create a complex chain of transforms with various rotations and translations
    // world -> base -> arm -> wrist -> gripper
    
    // Create a rotation quaternion for 90 degrees around Z
    let rot_z_90 = Quaternion {
        w: (std::f64::consts::PI / 4.0).cos(),
        x: 0.0,
        y: 0.0,
        z: (std::f64::consts::PI / 4.0).sin(),
    };
    
    // Transform from world to base (translation only)
    let t_world_base = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "world".into(),
        child: "base".into(),
    };
    
    // Transform from base to arm (rotation around Z)
    let t_base_arm = Transform {
        translation: Vector3::new(0.0, 1.0, 0.0),
        rotation: rot_z_90,
        timestamp: t,
        parent: "base".into(),
        child: "arm".into(),
    };
    
    // Transform from arm to wrist
    let t_arm_wrist = Transform {
        translation: Vector3::new(0.5, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "arm".into(),
        child: "wrist".into(),
    };
    
    // Transform from wrist to gripper
    let t_wrist_gripper = Transform {
        translation: Vector3::new(0.25, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "wrist".into(),
        child: "gripper".into(),
    };
    
    // Add all transforms to registry
    registry.add_transform(t_world_base);
    registry.add_transform(t_base_arm);
    registry.add_transform(t_arm_wrist);
    registry.add_transform(t_wrist_gripper);
    
    // Try to get transform from world to gripper
    let result = registry.get_transform("world", "gripper", t);
    assert!(result.is_ok(), "Failed to get transform from world to gripper");
    
    let transform = result.unwrap();
    debug!("World to gripper transform: {:?}", transform);
    
    // Verify the transform is as expected (simplified check)
    assert_eq!(transform.parent, "world");
    assert_eq!(transform.child, "gripper");
}

#[test]
fn test_multi_level_interpolation() {
    let _ = env_logger::try_init();

    #[cfg(not(feature = "std"))]
    let mut registry = Registry::new();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    #[cfg(feature = "std")]
    let mut registry = Registry::new(Duration::from_secs(10));
    #[cfg(feature = "std")]
    let t = Timestamp::now();
    
    // Create two sets of transforms at different times
    let t0 = t;
    let t1 = (t + Duration::from_secs(2)).unwrap();
    
    // First set at t0
    let t_a_b_0 = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t0,
        parent: "a".into(),
        child: "b".into(),
    };
    
    let t_b_c_0 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t0,
        parent: "b".into(),
        child: "c".into(),
    };
    
    // Second set at t1
    let t_a_b_1 = Transform {
        translation: Vector3::new(0.0, 2.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t1,
        parent: "a".into(),
        child: "b".into(),
    };
    
    let t_b_c_1 = Transform {
        translation: Vector3::new(1.0, 0.0, 2.0),
        rotation: Quaternion::identity(),
        timestamp: t1,
        parent: "b".into(),
        child: "c".into(),
    };
    
    registry.add_transform(t_a_b_0);
    registry.add_transform(t_b_c_0);
    registry.add_transform(t_a_b_1);
    registry.add_transform(t_b_c_1);
    
    // Get transform at intermediate time
    let t_mid = (t + Duration::from_secs(1)).unwrap();
    let result = registry.get_transform("a", "c", t_mid);
    
    assert!(result.is_ok(), "Failed to get interpolated multi-level transform");
    
    let transform = result.unwrap();
    debug!("Interpolated a->c transform at t_mid: {:?}", transform);
    
    // Should be linearly interpolated in both chains
    assert_eq!(transform.parent, "a");
    assert_eq!(transform.child, "c");
    assert_relative_eq!(transform.translation.x, 1.0, epsilon = 0.01);
    assert_relative_eq!(transform.translation.y, 1.0, epsilon = 0.01);
    assert_relative_eq!(transform.translation.z, 1.0, epsilon = 0.01);
}