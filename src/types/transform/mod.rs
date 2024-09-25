use crate::types::{Duration, Quaternion, Timestamp, Vector3};
use core::ops::Mul;
use std::f64::EPSILON;

mod error;
use error::TransformError;

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
    type Output = Result<Transform, TransformError>;

    #[inline]
    fn mul(
        self,
        rhs: Transform,
    ) -> Self::Output {
        let duration = if self.timestamp > rhs.timestamp {
            self.timestamp - rhs.timestamp
        } else {
            rhs.timestamp - self.timestamp
        };

        if duration.as_seconds() > 2 * EPSILON {
            return Err(TransformError::TimestampMismatch(duration.as_seconds()));
        }

        if self.frame == rhs.frame {
            return Err(TransformError::SameFrameMultiplication);
        }

        if self.frame != rhs.parent && self.parent != rhs.frame {
            return Err(TransformError::IncompatibleFrames);
        }

        let r = self.rotation * rhs.rotation;
        let t = self.translation + self.rotation.rotate_vector(&rhs.translation);
        let d = duration;

        Ok(Transform {
            translation: t,
            rotation: r,
            timestamp: self.timestamp + d / 2.0,
            frame: self.frame,
            parent: rhs.parent,
        })
    }
}

#[cfg(test)]
mod tests;
