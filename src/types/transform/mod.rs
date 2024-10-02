use crate::types::{Quaternion, Timestamp, Vector3};
use core::ops::Mul;
use std::f64::EPSILON;

mod error;
pub use error::TransformError;

#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub translation: Vector3,
    pub rotation: Quaternion,
    pub timestamp: Timestamp,
    pub parent: String,
    pub child: String,
}

impl Transform {
    pub fn interpolate(
        from: Transform,
        to: Transform,
        timestamp: Timestamp,
    ) -> Result<Transform, TransformError> {
        if from.timestamp > to.timestamp || timestamp < from.timestamp || timestamp > to.timestamp {
            return Err(TransformError::TimestampMismatch(
                to.timestamp.as_seconds()?,
                from.timestamp.as_seconds()?,
            ));
        }
        if from.child != to.child || from.parent != to.parent {
            return Err(TransformError::IncompatibleFrames);
        }

        let range = to.timestamp.nanoseconds - from.timestamp.nanoseconds;
        if range == 0 {
            return Ok(from);
        }

        let diff = timestamp.nanoseconds - from.timestamp.nanoseconds;
        let ratio = diff as f64 / range as f64;
        Ok(Transform {
            translation: (1.0 - ratio) * from.translation + ratio * to.translation,
            rotation: from.rotation.slerp(to.rotation, ratio),
            timestamp,
            child: from.child,
            parent: from.parent,
        })
    }

    pub fn identity() -> Self {
        Transform {
            translation: Vector3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            rotation: Quaternion {
                w: 1.0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            timestamp: Timestamp { nanoseconds: 0 },
            parent: "".to_string(),
            child: "".to_string(),
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
            (self.timestamp - rhs.timestamp)?
        } else {
            (rhs.timestamp - self.timestamp)?
        };

        if duration.as_seconds()? > 2.0 * EPSILON {
            return Err(TransformError::TimestampMismatch(
                self.timestamp.as_seconds()?,
                rhs.timestamp.as_seconds()?,
            ));
        }

        if self.child == rhs.child {
            return Err(TransformError::SameFrameMultiplication);
        }

        if self.child != rhs.parent && self.parent != rhs.child {
            return Err(TransformError::IncompatibleFrames);
        }

        let r = self.rotation * rhs.rotation;
        let t = self.rotation.rotate_vector(rhs.translation) + self.translation;
        let d = duration;

        Ok(Transform {
            translation: t,
            rotation: r,
            timestamp: (self.timestamp + d / 2.0)?,
            parent: rhs.parent,
            child: self.child,
        })
    }
}

impl Transform {
    pub fn inverse(&self) -> Result<Transform, TransformError> {
        let inverse_rotation = self.rotation.conjugate();
        let inverse_translation = -1.0 * (inverse_rotation.rotate_vector(self.translation));

        Ok(Transform {
            translation: inverse_translation,
            rotation: inverse_rotation,
            timestamp: self.timestamp,
            parent: self.child.clone(),
            child: self.parent.clone(),
        })
    }
}

#[cfg(test)]
mod tests;
