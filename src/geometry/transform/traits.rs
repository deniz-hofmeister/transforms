use crate::geometry::transform::{Transform, TransformError};
/// A trait for types that can be transformed between different coordinate frames.
///
/// This trait provides functionality to apply spatial transformations to objects,
/// typically used in robotics and computer vision applications. The transformations
/// follow the common robotics convention where transforms are considered from child
/// to parent frame (e.g., from sensor frame to base frame, or from base frame to
/// map frame).
///
/// # Frame Convention
///
/// In robotics, it's common to transform data from sensor reference frames "up" to
/// base or map reference frames. For example:
/// - A camera's data might need to be transformed from the camera frame to the robot's base frame
/// - Lidar points might need to be transformed from the lidar frame to the map frame
///
/// This trait follows this convention, where transforms are applied from child frame
/// to parent frame. The child frame is typically the more specific/local frame (e.g.,
/// a sensor frame), while the parent frame is typically the more general/global frame
/// (e.g., map or world frame).
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Point, Quaternion, Transform, Transformable, Vector3},
///     time::Timestamp,
/// };
///
/// let mut point = Point {
///     position: Vector3::new(1.0, 0.0, 0.0),
///     orientation: Quaternion::identity(),
///     timestamp: Timestamp::zero(),
///     frame: "camera".into(),
/// };
///
/// let transform = Transform {
///     translation: Vector3::new(0.0, 1.0, 0.0),
///     rotation: Quaternion::identity(),
///     timestamp: point.timestamp,
///     parent: "base".into(),
///     child: "camera".into(),
/// };
///
/// // Transform the point from camera frame to base frame
/// point
///     .transform(&transform)
///     .expect("Failed to transform point");
/// ```
///
/// # Errors
///
/// Returns `TransformError` if:
/// - The frames are incompatible (transform's child frame doesn't match the object's frame)
/// - The timestamps don't match
/// - Other transform-specific errors occur
pub trait Transformable {
    /// Applies a transform to this object, modifying it in place.
    ///
    /// # Arguments
    ///
    /// * `transform` - The transform to apply to this object
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the transformation was successful
    /// * `Err(TransformError)` if the transformation failed
    ///
    /// # Errors
    ///
    /// This method returns a `TransformError` if:
    /// - The frames of the object and the transform are incompatible.
    /// - The timestamps of the object and the transform do not match.
    fn transform(
        &mut self,
        transform: &Transform,
    ) -> Result<(), TransformError>;
}
