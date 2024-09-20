use crate::types::{Timestamp, Transform};
use std::collections::BTreeMap;
use std::rc::Rc;

/// A time-ordered collection of `Transform` objects.
///
/// The `Buffer` struct stores `Transform` objects indexed by their `Timestamp`.
/// It supports efficient insertion, retrieval, and cleanup of points based on their timestamps.
/// The `max_age` field specifies the maximum age of points to retain, in nanoseconds.
pub struct Buffer {
    data: BTreeMap<Timestamp, Rc<Transform>>,
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
        transform: Rc<Transform>,
    ) {
        self.data.insert(transform.timestamp, transform);
    }

    pub fn get(
        &self,
        timestamp: &Timestamp,
    ) -> Option<&Rc<Transform>> {
        self.data.get(timestamp)
    }

    pub fn get_nearest(
        &self,
        timestamp: &Timestamp,
    ) -> (
        Option<(&Timestamp, &Rc<Transform>)>,
        Option<(&Timestamp, &Rc<Transform>)>,
    ) {
        let before = self.data.range(..=timestamp).next_back();

        if let Some((t, _)) = before {
            if t == timestamp {
                return (before, before);
            }
        }

        let after = self.data.range(timestamp..).next();
        (before, after)
    }

    pub fn delete_before(
        &mut self,
        timestamp: Timestamp,
    ) {
        self.data.retain(|&k, _| k >= timestamp);
    }

    pub fn delete_expired(&mut self) {
        let timestamp_threshold = Timestamp::now() - self.max_age;
        self.delete_before(timestamp_threshold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Quaternion, Timestamp, Vector3};
    use std::rc::Rc;

    fn create_transform(ns: u128) -> Rc<Transform> {
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
        let frame = "map";
        let parent = None;
        Transform::new(translation, rotation, timestamp, frame, parent)
    }

    #[test]
    fn insert_and_get() {
        let mut buffer = Buffer::new(u128::MAX);
        let transform = create_transform(1000);
        buffer.insert(transform.clone());

        let mut r = buffer.get(&transform.timestamp);

        assert!(r.is_some(), "transform not found");
        assert!(Rc::ptr_eq(r.unwrap(), &transform));

        r = buffer.get(&Timestamp { nanoseconds: 999 });
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
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p2.timestamp, Rc::as_ptr(&p2)))
        );
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p2.timestamp, Rc::as_ptr(&p2)))
        );

        // Between two points
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1500 });
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p1.timestamp, Rc::as_ptr(&p1)))
        );
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p2.timestamp, Rc::as_ptr(&p2)))
        );

        // Before first point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 500 });
        assert!(before.is_none());
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p1.timestamp, Rc::as_ptr(&p1)))
        );

        // After last point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 3500 });
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p3.timestamp, Rc::as_ptr(&p3)))
        );
        assert!(after.is_none());

        // Exactly at first point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p1.timestamp, Rc::as_ptr(&p1)))
        );
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p1.timestamp, Rc::as_ptr(&p1)))
        );

        // Exactly at last point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 3000 });
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p3.timestamp, Rc::as_ptr(&p3)))
        );
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&p3.timestamp, Rc::as_ptr(&p3)))
        );
    }

    #[test]
    fn delete_before() {
        let mut buffer = Buffer::new(u128::MAX);
        buffer.insert(create_transform(1000));
        buffer.insert(create_transform(2000));
        buffer.insert(create_transform(3000));

        buffer.delete_before(Timestamp { nanoseconds: 2000 });

        assert!(buffer.get(&Timestamp { nanoseconds: 1000 }).is_none());
        assert!(buffer.get(&Timestamp { nanoseconds: 2000 }).is_some());
        assert!(buffer.get(&Timestamp { nanoseconds: 3000 }).is_some());
    }

    #[test]
    fn empty_buffer() {
        let buffer = Buffer::new(u128::MAX);
        assert!(buffer.get(&Timestamp { nanoseconds: 1000 }).is_none());

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
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&point.timestamp, Rc::as_ptr(&point)))
        );

        // Exact match
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1000 });
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&point.timestamp, Rc::as_ptr(&point)))
        );
        assert_eq!(
            after.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&point.timestamp, Rc::as_ptr(&point)))
        );

        // After the point
        let (before, after) = buffer.get_nearest(&Timestamp { nanoseconds: 1500 });
        assert_eq!(
            before.map(|(ts, t)| (ts, Rc::as_ptr(t))),
            Some((&point.timestamp, Rc::as_ptr(&point)))
        );
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

        assert!(buffer.get(&old_point.timestamp).is_none());
        assert!(buffer.get(&recent_point.timestamp).is_some());
    }
}
