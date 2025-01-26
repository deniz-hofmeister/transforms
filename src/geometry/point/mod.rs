use crate::{
    errors::TransformError,
    geometry::{Quaternion, Vector3},
    time::Timestamp,
    Transform, Transformable,
};

use alloc::string::String;

mod error;

/// Represents a point in space with a position, orientation, and timestamp.
///
/// The `Point` struct encapsulates a 3D position using a `Vector3`, an orientation
/// using a `Quaternion`, and a `Timestamp` to indicate when the point was recorded.
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Point, Quaternion, Vector3},
///     time::Timestamp,
/// };
///
/// let position = Vector3 {
///     x: 1.0,
///     y: 2.0,
///     z: 3.0,
/// };
/// let orientation = Quaternion {
///     w: 1.0,
///     x: 0.0,
///     y: 0.0,
///     z: 0.0,
/// };
/// let timestamp = Timestamp::zero();
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
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Point {
    pub position: Vector3,
    pub orientation: Quaternion,
    pub timestamp: Timestamp,
    pub frame: String,
}

/// The `Transformable` trait defines an interface for objects that can be transformed
/// using a `Transform`. Implementors of this trait can apply a transformation to
/// themselves, modifying their position and orientation.
///
/// Below is an example of how to implement the `Transformable` trait for a `Point`.
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Point, Quaternion, Vector3},
///     time::Timestamp,
///     Transform, Transformable,
/// };
///
/// let position = Vector3 {
///     x: 1.0,
///     y: 2.0,
///     z: 3.0,
/// };
/// let orientation = Quaternion {
///     w: 1.0,
///     x: 0.0,
///     y: 0.0,
///     z: 0.0,
/// };
/// let timestamp = Timestamp { t: 0 };
/// let frame = String::from("b");
///
/// let mut point = Point {
///     position,
///     orientation,
///     timestamp,
///     frame,
/// };
///
/// let transform = Transform {
///     translation: Vector3 {
///         x: 2.0,
///         y: 0.0,
///         z: 0.0,
///     },
///     rotation: Quaternion {
///         w: 1.0,
///         x: 0.0,
///         y: 0.0,
///         z: 0.0,
///     },
///     timestamp: Timestamp { t: 0 },
///     parent: "a".into(),
///     child: "b".into(),
/// };
/// let r = point.transform(&transform);
/// assert!(r.is_ok());
/// assert_eq!(point.frame, "a");
/// assert_eq!(point.position.x, 3.0);
/// ```
impl Transformable for Point {
    /// Applies a transformation to the `Point`.
    ///
    /// # Arguments
    ///
    /// * `transform` - A reference to the `Transform` to be applied.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transformation is successfully applied.
    /// * `Err(TransformError)` if the frames are incompatible or the timestamps do not match.
    fn transform(
        &mut self,
        transform: &Transform,
    ) -> Result<(), TransformError> {
        if self.frame != transform.child {
            return Err(TransformError::IncompatibleFrames);
        }
        if self.timestamp != transform.timestamp {
            #[allow(clippy::cast_precision_loss)]
            return Err(TransformError::TimestampMismatch(
                self.timestamp.t as f64,
                transform.timestamp.t as f64,
            ));
        }
        self.position = transform.rotation.rotate_vector(self.position) + transform.translation;
        self.orientation = transform.rotation * self.orientation;
        self.frame.clone_from(&transform.parent);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
