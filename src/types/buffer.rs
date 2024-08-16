use crate::types::{Point, Timestamp};
use std::collections::BTreeMap;

/// A time-ordered collection of `Point` objects.
///
/// The `Buffer` struct stores `Point` objects indexed by their `Timestamp`.
/// It supports efficient insertion, retrieval, and cleanup of points based on their timestamps.
/// The `max_age` field specifies the maximum age of points to retain, in nanoseconds.
pub struct Buffer {
    data: BTreeMap<Timestamp, Point>,
    max_age: u128,
}

impl Buffer {
    pub fn new(max_age: u128) -> Self {
        Self {
            data: BTreeMap::new(),
            max_age,
        }
    }

    pub fn insert(
        &mut self,
        point: Point,
    ) {
        self.data
            .insert(point.timestamp, point);
    }

    pub fn get(
        &self,
        timestamp: &Timestamp,
    ) -> Option<&Point> {
        self.data
            .get(timestamp)
    }

    pub fn get_nearest(
        &self,
        timestamp: &Timestamp,
    ) -> (Option<(&Timestamp, &Point)>, Option<(&Timestamp, &Point)>) {
        let before = self
            .data
            .range(..=timestamp)
            .next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = self
            .data
            .range(timestamp..)
            .next();
        (before, after)
    }

    pub fn delete_before(
        &mut self,
        timestamp: Timestamp,
    ) {
        self.data
            .retain(|&k, _| k >= timestamp);
    }

    pub fn delete_expired(&mut self) {
        let timestamp_threshold = Timestamp::now() - self.max_age;
        self.delete_before(timestamp_threshold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Point, Quaternion, Timestamp, Vector3};

    fn create_point(ns: u128) -> Point {
        Point {
            position: Vector3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            orientation: Quaternion {
                w: 1.0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            timestamp: Timestamp { nanoseconds: ns },
        }
    }

    #[test]
    fn insert_and_get() {
        let mut buffer = Buffer::new(u128::MAX);
        let point = create_point(1000);
        buffer.insert(point.clone());

        assert_eq!(buffer.get(&point.timestamp), Some(&point));
        assert_eq!(buffer.get(&Timestamp { nanoseconds: 999 }), None);
    }

    #[test]
    fn get_nearest() {
        let mut buffer = Buffer::new(u128::MAX);
        let p1 = create_point(1000);
        let p2 = create_point(2000);
        let p3 = create_point(3000);

        buffer.insert(p1.clone());
        buffer.insert(p2.clone());
        buffer.insert(p3.clone());

        // Exact match
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 2000 });
        assert_eq!(before, Some((&p2.timestamp, &p2)));
        assert_eq!(after, Some((&p2.timestamp, &p2)));

        // Between two points
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1500 });
        assert_eq!(before, Some((&p1.timestamp, &p1)));
        assert_eq!(after, Some((&p2.timestamp, &p2)));

        // Before first point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 500 });
        assert_eq!(before, None);
        assert_eq!(after, Some((&p1.timestamp, &p1)));

        // After last point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 3500 });
        assert_eq!(before, Some((&p3.timestamp, &p3)));
        assert_eq!(after, None);

        // Exactly at first point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(before, Some((&p1.timestamp, &p1)));
        assert_eq!(after, Some((&p1.timestamp, &p1)));
        // Exactly at last point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 3000 });
        assert_eq!(before, Some((&p3.timestamp, &p3)));
        assert_eq!(after, Some((&p3.timestamp, &p3)));
    }

    #[test]
    fn delete_before() {
        let mut buffer = Buffer::new(u128::MAX);
        buffer.insert(create_point(1000));
        buffer.insert(create_point(2000));
        buffer.insert(create_point(3000));

        buffer.delete_before(Timestamp { nanoseconds: 2000 });

        assert_eq!(buffer.get(&Timestamp { nanoseconds: 1000 }), None);
        assert_eq!(
            buffer.get(&Timestamp { nanoseconds: 2000 }),
            Some(&create_point(2000))
        );
        assert_eq!(
            buffer.get(&Timestamp { nanoseconds: 3000 }),
            Some(&create_point(3000))
        );
    }

    #[test]
    fn empty_buffer() {
        let buffer = Buffer::new(u128::MAX);
        assert_eq!(buffer.get(&Timestamp { nanoseconds: 1000 }), None);

        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(before, None);
        assert_eq!(after, None);
    }

    #[test]
    fn single_point_buffer() {
        let mut buffer = Buffer::new(0);
        let point = create_point(1000);
        buffer.insert(point.clone());

        // Before the point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 500 });
        assert_eq!(before, None);
        assert_eq!(after, Some((&point.timestamp, &point)));

        // Exact match
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(before, Some((&point.timestamp, &point)));
        assert_eq!(after, Some((&point.timestamp, &point)));

        // After the point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1500 });
        assert_eq!(before, Some((&point.timestamp, &point)));
        assert_eq!(after, None);
    }

    #[test]
    fn delete_expired() {
        let mut buffer = Buffer::new(2_000_000_000);

        let now = Timestamp::now();
        let old_point = create_point(now.nanoseconds - 3_000_000_000);
        let recent_point = create_point(now.nanoseconds - 1_000_000_000);

        buffer.insert(old_point.clone());
        buffer.insert(recent_point.clone());

        buffer.delete_expired();

        assert_eq!(buffer.get(&old_point.timestamp), None);
        assert_eq!(buffer.get(&recent_point.timestamp), Some(&recent_point));
    }
}
