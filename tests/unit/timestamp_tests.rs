use std::time::Duration;
use transforms::time::Timestamp;
use transforms::errors::TimestampError;

#[test]
fn test_timestamp_creation() {
    // Test zero timestamp
    let t_zero = Timestamp::zero();
    assert_eq!(t_zero.t, 0);
    
    // Test timestamp with specific value
    let t_value = Timestamp { t: 1000000000 }; // 1 second in nanoseconds
    assert_eq!(t_value.t, 1000000000);
    
    #[cfg(feature = "std")]
    {
        // Test now() function (std feature only)
        let t_now = Timestamp::now();
        assert!(t_now.t > 0, "Current timestamp should be positive");
    }
}

#[test]
fn test_timestamp_math() {
    let t1 = Timestamp { t: 1000000000 }; // 1 second
    let t2 = Timestamp { t: 2000000000 }; // 2 seconds
    
    // Test addition
    let result = (t1 + Duration::from_millis(500)).unwrap();
    assert_eq!(result.t, 1500000000); // 1.5 seconds
    
    // Test subtraction
    let result = (t2 - Duration::from_millis(500)).unwrap();
    assert_eq!(result.t, 1500000000); // 1.5 seconds
    
    // Test Duration between timestamps
    let duration = (t2 - t1).unwrap();
    assert_eq!(duration.as_nanos(), 1000000000); // 1 second
    
    // Test negative duration error
    let result = t1 - t2;
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TimestampError::NegativeDuration));
}

#[test]
fn test_timestamp_comparison() {
    let t1 = Timestamp { t: 1000000000 }; // 1 second
    let t2 = Timestamp { t: 2000000000 }; // 2 seconds
    let t3 = Timestamp { t: 1000000000 }; // Same as t1
    
    // Test equality
    assert_eq!(t1, t3);
    assert_ne!(t1, t2);
    
    // Test ordering
    assert!(t1 < t2);
    assert!(t2 > t1);
    assert!(t1 <= t3);
    assert!(t1 >= t3);
}

#[test]
fn test_timestamp_seconds_conversion() {
    let t = Timestamp { t: 1500000000 }; // 1.5 seconds
    
    let seconds = t.as_seconds();
    assert!(seconds.is_ok());
    assert_eq!(seconds.unwrap(), 1.5);
    
    // Test very large timestamp
    let t_large = Timestamp { t: 9223372036854775000 }; // Near i64::MAX
    let seconds_large = t_large.as_seconds();
    assert!(seconds_large.is_ok());
    assert!(seconds_large.unwrap() > 9.22e18);
}

#[test]
fn test_timestamp_edge_cases() {
    // Test zero timestamp
    let t_zero = Timestamp::zero();
    
    // Adding to zero
    let result = (t_zero + Duration::from_secs(10)).unwrap();
    assert_eq!(result.t, 10000000000);
    
    // Subtracting from zero (should fail)
    let result = t_zero - Duration::from_secs(10);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TimestampError::NegativeDuration));
    
    // Test overflow scenarios with large durations
    let t = Timestamp { t: 9223372036854775000 }; // Near i64::MAX
    let result = t + Duration::from_secs(1000);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), TimestampError::OverflowError));
}

#[test]
fn test_timestamp_clone() {
    let t1 = Timestamp { t: 1000000000 };
    let t2 = t1.clone();
    
    assert_eq!(t1, t2);
    
    // Verify they're separate instances
    let t3 = (t2 + Duration::from_secs(1)).unwrap();
    assert_ne!(t1, t3);
}

#[test]
fn test_timestamp_display() {
    let t = Timestamp { t: 1500000000 }; // 1.5 seconds in nanoseconds
    
    // Use format to get the displayed string
    let displayed = format!("{}", t);
    
    // The display should show the timestamp value
    assert!(displayed.contains("1500000000"), "Display should include the timestamp value");
}

#[test]
fn test_timestamp_div_f64() {
    let duration = Duration::from_nanos(10000000000); // 10 seconds
    
    // Divide by 2.0
    let half_duration = duration.div_f64(2.0);
    assert_eq!(half_duration.as_nanos(), 5000000000); // 5 seconds
    
    // Divide by 0.5 (double)
    let double_duration = duration.div_f64(0.5);
    assert_eq!(double_duration.as_nanos(), 20000000000); // 20 seconds
    
    // Divide by very small value (should result in very large duration)
    let large_duration = duration.div_f64(0.000001);
    assert_eq!(large_duration.as_nanos(), 10000000000000000); // 10^16 nanoseconds
}

#[test]
fn test_timestamp_from_various_durations() {
    let t_base = Timestamp::zero();
    
    // Test various durations
    let durations = [
        Duration::from_nanos(1),
        Duration::from_micros(1),
        Duration::from_millis(1),
        Duration::from_secs(1),
        Duration::from_secs(60),
        Duration::from_secs(3600),
        Duration::from_secs(86400), // 1 day
    ];
    
    for duration in durations {
        let result = t_base + duration;
        assert!(result.is_ok(), "Failed to add duration: {:?}", duration);
        
        let t_new = result.unwrap();
        assert_eq!(t_new.t, duration.as_nanos() as i64);
    }
}