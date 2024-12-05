use crate::types::{Quaternion, Timestamp, Vector3};
mod error;

/// Represents a point in space with a position, orientation, and timestamp.
///
/// The `Point` struct encapsulates a 3D position using a `Vector3`, an orientation
/// using a `Quaternion`, and a `Timestamp` to indicate when the point was recorded.
///
/// # Examples
///
/// ```
/// # use transforms::types::{Quaternion, Timestamp, Vector3, Point};
///
/// let position = Vector3 { x: 1.0, y: 2.0, z: 3.0 };
/// let orientation = Quaternion { w: 1.0, x: 0.0, y: 0.0, z: 0.0 };
/// let timestamp = Timestamp::now();
/// let frame = String::from("a");
///
/// let point = Point {
///     position,
///     orientation,
///     timestamp,
///     frame,
/// };
///
/// assert_eq!(point.position.x, 1.0);
/// assert_eq!(point.orientation.w, 1.0);
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Point {
    pub position: Vector3,
    pub orientation: Quaternion,
    pub timestamp: Timestamp,
    pub frame: String,
}

#[cfg(test)]
mod tests;
