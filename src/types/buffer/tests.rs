#[cfg(test)]
mod tests {
    use crate::types::{Buffer, Quaternion, Timestamp, Transform, Vector3};

    fn create_transform(ns: u128) -> Transform {
        let translation = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let rotation = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let timestamp = Timestamp { nanoseconds: ns };
        let frame = "base";
        let parent = "map";
        Transform::new(translation, rotation, timestamp, frame, parent)
    }

    #[test]
    fn insert_and_get() {
        let mut buffer = Buffer::new(u128::MAX);
        let transform = create_transform(1000);
        buffer.insert(transform.clone());

        let mut r = buffer.get_exact(&transform.timestamp);

        assert!(r.is_some(), "transform not found");
        assert_eq!(r.unwrap(), &transform);

        r = buffer.get_exact(&Timestamp { nanoseconds: 999 });
        assert!(r.is_none(), "transform found, but shouldnt't have");
    }

    #[test]
    fn get_nearest() {
        let mut buffer = Buffer::new(u128::MAX);
        let p1 = create_transform(1000);
        let p2 = create_transform(2000);
        let p3 = create_transform(3000);

        buffer.insert(p1.clone());
        buffer.insert(p2.clone());
        buffer.insert(p3.clone());

        // Exact match
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 2000 });
        assert_eq!(before.unwrap(), ((&p2.timestamp, &p2)));
        assert_eq!(after.unwrap(), ((&p2.timestamp, &p2)));

        // Between two points
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1500 });
        assert_eq!(before.unwrap(), ((&p1.timestamp, &p1)));
        assert_eq!(after.unwrap(), ((&p2.timestamp, &p2)));

        // Before first point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 500 });
        assert_eq!(before, None);
        assert_eq!(after.unwrap(), ((&p1.timestamp, &p1)));

        // After last point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 3500 });
        assert_eq!(before.unwrap(), (&p3.timestamp, &p3));
        assert_eq!(after, None);

        // Exactly at first point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(before.unwrap(), (&p1.timestamp, &p1));
        assert_eq!(after.unwrap(), (&p1.timestamp, &p1));

        // Exactly at last point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 3000 });
        assert_eq!(before.unwrap(), (&p3.timestamp, &p3));
        assert_eq!(after.unwrap(), (&p3.timestamp, &p3));
    }

    #[test]
    fn delete_before() {
        let mut buffer = Buffer::new(u128::MAX);
        buffer.insert(create_transform(1000));
        buffer.insert(create_transform(2000));
        buffer.insert(create_transform(3000));

        buffer.delete_before(Timestamp { nanoseconds: 2000 });

        assert!(buffer.get_exact(&Timestamp { nanoseconds: 1000 }).is_none());
        assert!(buffer.get_exact(&Timestamp { nanoseconds: 2000 }).is_some());
        assert!(buffer.get_exact(&Timestamp { nanoseconds: 3000 }).is_some());
    }

    #[test]
    fn empty_buffer() {
        let buffer = Buffer::new(u128::MAX);
        assert!(buffer.get_exact(&Timestamp { nanoseconds: 1000 }).is_none());

        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert!(before.is_none());
        assert!(after.is_none());
    }

    #[test]
    fn single_point_buffer() {
        let mut buffer = Buffer::new(0);
        let point = create_transform(1000);
        buffer.insert(point.clone());

        // Before the point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 500 });
        assert!(before.is_none());
        assert_eq!(after.unwrap(), (&point.timestamp, &point));

        // Exact match
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(before.unwrap(), (&point.timestamp, &point));
        assert_eq!(after.unwrap(), (&point.timestamp, &point));

        // After the point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1500 });
        assert_eq!(before.unwrap(), (&point.timestamp, &point));
        assert!(after.is_none());
    }

    #[test]
    fn delete_expired() {
        let mut buffer = Buffer::new(2_000_000_000);

        let now = Timestamp::now();
        let old_point = create_transform(now.nanoseconds - 3_000_000_000);
        let recent_point = create_transform(now.nanoseconds - 1_000_000_000);

        buffer.insert(old_point.clone());
        buffer.insert(recent_point.clone());

        buffer.delete_expired();

        assert!(buffer.get_exact(&old_point.timestamp).is_none());
        assert!(buffer.get_exact(&recent_point.timestamp).is_some());
    }
}
