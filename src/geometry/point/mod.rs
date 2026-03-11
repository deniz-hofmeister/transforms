use crate::{
    errors::TransformError,
    geometry::{Quaternion, Vector3},
    time::{TimePoint, Timestamp},
    Localized, Transform, Transformable,
};

use alloc::string::String;

mod error;

/// Represents a point in space with a position, orientation, timestamp, and its frame of reference.
///
/// The `Point` struct represents a single observation of data, at some given moment in time, with respect
/// to a specific reference frame. It encapsulates a 3D position using a `Vector3`, an orientation
/// using a `Quaternion`, a `Timestamp` to indicate when the point was recorded, and  a `String`
/// representing the coordinate reference frame its data is relative to.
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
pub struct Point<T = Timestamp>
where
    T: TimePoint,
{
    pub position: Vector3,
    pub orientation: Quaternion,
    pub timestamp: T,
    pub frame: String,
}

/// The `Transformable` trait defines an interface for objects that can be transformed
/// using a `Transform`. Implementors of this trait can apply a transformation to
/// themselves, modifying their position and orientation.
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
/// let mut point = Point {
///     position: Vector3::new(1.0, 2.0, 3.0),
///     orientation: Quaternion::identity(),
///     timestamp: Timestamp::zero(),
///     frame: "b".into(),
/// };
///
/// let transform = Transform {
///     translation: Vector3::new(2.0, 0.0, 0.0),
///     rotation: Quaternion::identity(),
///     timestamp: Timestamp::zero(),
///     parent: "a".into(),
///     child: "b".into(),
/// };
///
/// let r = point.transform(&transform);
/// assert!(r.is_ok());
/// assert_eq!(point.frame, "a");
/// assert_eq!(point.position.x, 3.0);
/// ```
impl<T> Transformable<T> for Point<T>
where
    T: TimePoint,
{
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
        transform: &Transform<T>,
    ) -> Result<(), TransformError> {
        if self.frame != transform.child {
            return Err(TransformError::IncompatibleFrames);
        }
        if self.timestamp != transform.timestamp {
            return Err(TransformError::TimestampMismatch(
                self.timestamp.as_seconds()?,
                transform.timestamp.as_seconds()?,
            ));
        }
        self.position = transform.rotation.rotate_vector(self.position) + transform.translation;
        self.orientation = transform.rotation * self.orientation;
        self.frame.clone_from(&transform.parent);
        Ok(())
    }
}

/// The `Localized` trait provides frame and timestamp introspection for a `Point`,
/// enabling automatic transform lookup via
/// [`Registry::get_transform_for`](crate::core::Registry::get_transform_for).
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")]
/// use core::time::Duration;
/// use transforms::{
///     geometry::{Point, Quaternion, Transform, Vector3},
///     time::Timestamp,
///     Registry, Transformable,
/// };
///
/// # #[cfg(feature = "std")]
/// let mut registry = Registry::new(Duration::from_secs(10));
/// # #[cfg(not(feature = "std"))]
/// # let mut registry = Registry::new();
/// # #[cfg(feature = "std")]
/// let t = Timestamp::now();
/// # #[cfg(not(feature = "std"))]
/// # let t = Timestamp::zero();
///
/// registry.add_transform(Transform {
///     translation: Vector3::new(1.0, 0.0, 0.0),
///     rotation: Quaternion::identity(),
///     timestamp: t,
///     parent: "map".into(),
///     child: "camera".into(),
/// });
///
/// let mut point = Point {
///     position: Vector3::new(1.0, 0.0, 0.0),
///     orientation: Quaternion::identity(),
///     timestamp: t,
///     frame: "camera".into(),
/// };
///
/// // Localized lets the registry extract frame and timestamp automatically
/// let tf = registry.get_transform_for(&point, "map").unwrap();
/// point.transform(&tf).unwrap();
/// assert_eq!(point.frame, "map");
/// assert_eq!(point.position.x, 2.0);
/// ```
impl<T> Localized<T> for Point<T>
where
    T: TimePoint,
{
    fn frame(&self) -> &str {
        &self.frame
    }

    fn timestamp(&self) -> T {
        self.timestamp
    }
}

#[cfg(test)]
mod tests;
