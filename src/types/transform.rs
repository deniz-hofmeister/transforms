use crate::types::{Quaternion, Timestamp, Vector3};
use core::ops::Mul;
use std::{error::Error, f64::EPSILON};

#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
    pub timestamp: Timestamp,
    pub frame: String,
    pub parent: String,
}

impl Transform {
    pub fn new(
        translation: Vector3,
        rotation: Quaternion,
        timestamp: Timestamp,
        frame: &str,
        parent: &str,
    ) -> Self {
        Transform {
            translation,
            rotation,
            timestamp,
            frame: frame.to_string(),
            parent: parent.to_string(),
        }
    }

    pub fn interpolate(
        from: Transform,
        to: Transform,
        timestamp: Timestamp,
    ) -> Transform {
        assert!(from.timestamp.nanoseconds <= to.timestamp.nanoseconds);
        assert!(from.timestamp.nanoseconds <= timestamp.nanoseconds);
        assert!(timestamp.nanoseconds <= to.timestamp.nanoseconds);
        assert_eq!(from.frame, to.frame);
        assert_eq!(from.parent, to.parent);

        let range = to.timestamp.nanoseconds - from.timestamp.nanoseconds;
        if range == 0 {
            return from;
        }

        let diff = timestamp.nanoseconds - from.timestamp.nanoseconds;
        let ratio = diff as f64 / range as f64;
        Transform {
            translation: (1.0 - ratio) * from.translation + ratio * to.translation,
            rotation: from.rotation.slerp(to.rotation, ratio),
            timestamp,
            frame: from.frame,
            parent: from.parent,
        }
    }
}

impl Mul for Transform {
    type Output = Transform;

    #[inline]
    fn mul(
        self,
        other: Transform,
    ) -> Result<Transform> {
        if (self.timestamp - other.timestamp) > EPSILON {
            Error
        }
        let t = self.translation + other.translation;
        let r = self.rotation * other.rotation;
        Ok(Transform {
            translation: t,
            rotation: r,
            timestamp: (self.timestamp + other.timestamp) / 2.0,
            frame: self.frame,
            parent: self.parent,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn transform_creation() {
        let v = Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let q = Quaternion {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let t = Timestamp::now();
        let f = "base";
        let p = "map";

        let _t = Transform::new(v, q, t, f, p);
    }
}
