use std::time::Duration;
use transforms::{
    core::Buffer,
    errors::BufferError,
    geometry::{Quaternion, Transform, Vector3},
    time::Timestamp,
};

#[test]
fn test_buffer_creation() {
    #[cfg(feature = "std")]
    let _buffer = Buffer::new(Duration::from_secs(60));

    #[cfg(not(feature = "std"))]
    let _buffer = Buffer::new();
}

#[test]
fn test_buffer_insert_get() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

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

    buffer.insert(transform.clone());
    
    let result = buffer.get(&t);
    assert!(result.is_ok(), "Failed to get transform that was just inserted");
    assert_eq!(result.unwrap(), &transform, "Retrieved transform doesn't match the inserted one");
}

#[test]
fn test_buffer_empty() {
    #[cfg(feature = "std")]
    let buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    let result = buffer.get(&t);
    assert!(matches!(result, Err(BufferError::EmptyBuffer)), "Expected EmptyBuffer error from empty buffer");
}

#[test]
fn test_buffer_timestamp_out_of_range() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t1 = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t1 = Timestamp::zero();

    let t2 = (t1 + Duration::from_secs(10)).unwrap();
    
    // Add two transforms a few seconds apart
    let transform1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t1,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let transform2 = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t2,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    buffer.insert(transform1);
    buffer.insert(transform2);
    
    // Try to get a transform with timestamp earlier than any in the buffer
    let early_time = (t1 - Duration::from_secs(1)).unwrap();
    let result = buffer.get(&early_time);
    assert!(matches!(result, Err(BufferError::TimestampOutOfRange)), "Expected TimestampOutOfRange for too early timestamp");
    
    // Try to get a transform with timestamp later than any in the buffer
    let late_time = (t2 + Duration::from_secs(1)).unwrap();
    let result = buffer.get(&late_time);
    assert!(matches!(result, Err(BufferError::TimestampOutOfRange)), "Expected TimestampOutOfRange for too late timestamp");
}

#[test]
fn test_buffer_interpolation() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t1 = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t1 = Timestamp::zero();

    let t2 = (t1 + Duration::from_secs(2)).unwrap();
    
    // Add two transforms 2 seconds apart
    let transform1 = Transform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t1,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let transform2 = Transform {
        translation: Vector3::new(10.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t2,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    buffer.insert(transform1);
    buffer.insert(transform2);
    
    // Get transform at midpoint (t1 + 1s)
    let mid_time = (t1 + Duration::from_secs(1)).unwrap();
    let result = buffer.get(&mid_time);
    
    assert!(result.is_ok(), "Failed to get interpolated transform");
    
    let interpolated = result.unwrap();
    assert_eq!(interpolated.translation.x, 5.0, "Expected x=5.0 at midpoint");
    assert_eq!(interpolated.timestamp, mid_time, "Expected timestamp to match requested time");
}

#[test]
fn test_buffer_exact_timestamp_match() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t1 = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t1 = Timestamp::zero();

    let t2 = (t1 + Duration::from_secs(1)).unwrap();
    let t3 = (t1 + Duration::from_secs(2)).unwrap();
    
    // Add multiple transforms
    let transform1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t1,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let transform2 = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t2,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let transform3 = Transform {
        translation: Vector3::new(3.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t3,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    buffer.insert(transform1.clone());
    buffer.insert(transform2.clone());
    buffer.insert(transform3.clone());
    
    // Get transforms at exact timestamps
    let result1 = buffer.get(&t1);
    let result2 = buffer.get(&t2);
    let result3 = buffer.get(&t3);
    
    assert!(result1.is_ok(), "Failed to get transform at t1");
    assert!(result2.is_ok(), "Failed to get transform at t2");
    assert!(result3.is_ok(), "Failed to get transform at t3");
    
    assert_eq!(result1.unwrap(), &transform1, "Transform at t1 doesn't match expected");
    assert_eq!(result2.unwrap(), &transform2, "Transform at t2 doesn't match expected");
    assert_eq!(result3.unwrap(), &transform3, "Transform at t3 doesn't match expected");
}

#[test]
fn test_buffer_static_transform() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    // Use timestamp zero for static transform
    let t_zero = Timestamp::zero();
    #[cfg(feature = "std")]
    let t_now = Timestamp::now();
    
    // Add a static transform (t=0)
    let static_transform = Transform {
        translation: Vector3::new(1.0, 2.0, 3.0),
        rotation: Quaternion::identity(),
        timestamp: t_zero,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    buffer.insert(static_transform.clone());
    
    // Should be able to get the static transform at any timestamp
    #[cfg(feature = "std")]
    let result = buffer.get(&t_now);
    #[cfg(not(feature = "std"))]
    let result = buffer.get(&(t_zero + Duration::from_secs(1000)).unwrap());
    
    assert!(result.is_ok(), "Failed to get static transform at different timestamp");
    assert_eq!(result.unwrap(), &static_transform, "Retrieved static transform doesn't match expected");
}

#[test]
fn test_buffer_delete_before() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t_base = Timestamp::now();
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

    buffer.insert(t1);
    buffer.insert(t2.clone());
    buffer.insert(t3.clone());
    
    // Delete transforms before t_base + 1s
    let delete_time = (t_base + Duration::from_secs(1)).unwrap();
    buffer.delete_before(delete_time);
    
    // The first transform should be gone, but the other two should remain
    let result1 = buffer.get(&t_base);
    let result2 = buffer.get(&(t_base + Duration::from_secs(1)).unwrap());
    let result3 = buffer.get(&(t_base + Duration::from_secs(2)).unwrap());
    
    assert!(result1.is_err(), "Transform before delete timestamp should be gone");
    assert!(result2.is_ok(), "Transform at delete timestamp should exist");
    assert!(result3.is_ok(), "Transform after delete timestamp should exist");
    
    assert_eq!(result2.unwrap(), &t2, "Retrieved transform doesn't match expected");
    assert_eq!(result3.unwrap(), &t3, "Retrieved transform doesn't match expected");
}

#[test]
fn test_buffer_multiple_inserts_same_timestamp() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t = Timestamp::zero();

    // Create two transforms with the same timestamp
    let transform1 = Transform {
        translation: Vector3::new(1.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    let transform2 = Transform {
        translation: Vector3::new(2.0, 0.0, 0.0),
        rotation: Quaternion::identity(),
        timestamp: t,
        parent: "parent".into(),
        child: "child".into(),
    };
    
    buffer.insert(transform1);
    buffer.insert(transform2.clone());
    
    // The second insert should overwrite the first
    let result = buffer.get(&t);
    
    assert!(result.is_ok(), "Failed to get transform");
    assert_eq!(result.unwrap(), &transform2, "Expected second transform to overwrite first");
}

#[test]
fn test_buffer_insert_order() {
    #[cfg(feature = "std")]
    let mut buffer = Buffer::new(Duration::from_secs(60));
    #[cfg(not(feature = "std"))]
    let mut buffer = Buffer::new();

    #[cfg(feature = "std")]
    let t_base = Timestamp::now();
    #[cfg(not(feature = "std"))]
    let t_base = Timestamp::zero();

    // Create transforms with different timestamps
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
    
    // Insert out of order
    buffer.insert(t3.clone());
    buffer.insert(t1.clone());
    buffer.insert(t2.clone());
    
    // Should be able to get all transforms correctly
    let result1 = buffer.get(&t_base);
    let result2 = buffer.get(&(t_base + Duration::from_secs(1)).unwrap());
    let result3 = buffer.get(&(t_base + Duration::from_secs(2)).unwrap());
    
    assert!(result1.is_ok(), "Failed to get transform at t_base");
    assert!(result2.is_ok(), "Failed to get transform at t_base+1s");
    assert!(result3.is_ok(), "Failed to get transform at t_base+2s");
    
    assert_eq!(result1.unwrap(), &t1, "Transform at t_base doesn't match expected");
    assert_eq!(result2.unwrap(), &t2, "Transform at t_base+1s doesn't match expected");
    assert_eq!(result3.unwrap(), &t3, "Transform at t_base+2s doesn't match expected");
}