//! Traits for locating values in a frame and applying transforms to them.

use crate::{
    geometry::transform::{Transform, TransformError},
    time::{TimePoint, Timestamp},
};

/// A trait for types that are localized in a specific coordinate frame at a specific time.
///
/// This trait provides frame and timestamp introspection, enabling automatic transform
/// lookup via [`Registry::get_transform_for`](crate::Registry::get_transform_for).
///
/// Separate from [`Transformable`] so that types without frame/timestamp metadata
/// can still implement `Transformable` independently.
///
/// # Examples
///
/// ```
/// use transforms::{
///     Localized,
///     geometry::{Point, Quaternion, Vector3},
///     time::Timestamp,
/// };
///
/// let point: Point = Point::new(
///     Vector3::new(1.0, 0.0, 0.0),
///     Quaternion::identity(),
///     Timestamp::zero(),
///     "camera",
/// );
///
/// assert_eq!(point.frame(), "camera");
/// ```
pub trait Localized<T = Timestamp>
where
    T: TimePoint,
{
    /// Returns the object's current frame identifier.
    fn frame(&self) -> &str;

    /// Returns the object's timestamp.
    fn timestamp(&self) -> T;
}

/// Applies a rigid-body transform to a value in place.
///
/// # Contract
///
/// For a transform `tf`, positions become
/// `tf.rotation().rotate_vector(position) + tf.translation()` and orientations
/// become `tf.rotation() * orientation`. Free vectors take only the rotation.
/// When present, the value's frame becomes `tf.parent()` and its timestamp is
/// preserved. [`Point`](crate::geometry::Point) is the reference implementation.
///
/// # Preconditions
///
/// The caller supplies numerically valid geometry. Implementations check frames
/// and timestamps, without re-validating numbers. Constructors validate
/// transforms; derived results may drift or overflow. Call
/// [`Transform::validate`] before applying a derived result when needed.
///
/// Cross-time results from
/// [`Registry::get_transform_at`](crate::Registry::get_transform_at) must be
/// applied explicitly when their source and target times differ; they retain
/// only the target stamp.
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Point, Quaternion, Transform, Transformable, Vector3},
///     time::{Stamp, Timestamp},
/// };
///
/// let mut point: Point = Point::new(
///     Vector3::new(1.0, 0.0, 0.0),
///     Quaternion::identity(),
///     Timestamp::zero(),
///     "camera",
/// );
///
/// let transform: Transform = Transform::new(
///     "base",
///     "camera",
///     Vector3::new(0.0, 1.0, 0.0),
///     Quaternion::identity(),
///     Stamp::At(point.timestamp),
/// )
/// .unwrap();
///
/// // Transform the point from camera frame to base frame
/// point.transform(&transform).unwrap();
/// assert_eq!(point.frame, "base");
/// assert_eq!(point.position, Vector3::new(1.0, 1.0, 0.0));
/// ```
///
/// Applying a transform to your own data needs no implementation of this
/// trait. For plain positions, these are the checks
/// [`Point`](crate::geometry::Point) performs:
///
/// ```
/// use transforms::{
///     geometry::{Quaternion, Transform, Vector3},
///     time::{Stamp, Timestamp},
/// };
///
/// struct Scan {
///     frame: String,
///     time: Timestamp,
///     positions: Vec<Vector3>,
/// }
///
/// fn apply(
///     tf: &Transform,
///     scan: &mut Scan,
/// ) -> Result<(), &'static str> {
///     if scan.frame != tf.child() {
///         return Err("the scan is not in the transform's child frame");
///     }
///     if tf.timestamp() != Stamp::Static && tf.timestamp() != Stamp::At(scan.time) {
///         return Err("the transform is for another instant");
///     }
///     for position in &mut scan.positions {
///         *position = tf.rotation().rotate_vector(*position) + tf.translation();
///     }
///     scan.frame = tf.parent().into();
///     Ok(())
/// }
///
/// let time = Timestamp::from_nanos(1);
/// let mut scan = Scan {
///     frame: "lidar".into(),
///     time,
///     positions: vec![Vector3::new(1.0, 0.0, 0.0)],
/// };
/// let tf = Transform::new(
///     "base",
///     "lidar",
///     Vector3::new(0.0, 1.0, 0.0),
///     Quaternion::identity(),
///     Stamp::At(time),
/// )
/// .unwrap();
///
/// apply(&tf, &mut scan).unwrap();
/// assert_eq!(scan.frame, "base");
/// assert_eq!(scan.positions, [Vector3::new(1.0, 1.0, 0.0)]);
/// // The scan has left the transform's child frame: applying twice is refused.
/// assert!(apply(&tf, &mut scan).is_err());
///
/// // So is a scan taken at another instant.
/// let mut later = Scan {
///     frame: "lidar".into(),
///     time: Timestamp::from_nanos(2),
///     positions: vec![Vector3::zero()],
/// };
/// assert_eq!(
///     apply(&tf, &mut later),
///     Err("the transform is for another instant")
/// );
/// ```
pub trait Transformable<T = Timestamp>
where
    T: TimePoint,
{
    /// Applies a transform to this object, modifying it in place.
    ///
    /// See the trait-level contract for position, orientation, and metadata updates.
    ///
    /// # Errors
    ///
    /// This method returns a `TransformError` if:
    /// - The frames of the object and the transform are incompatible.
    /// - The timestamps of the object and the transform do not match; static
    ///   transforms (`Stamp::Static`) are exempt, being valid for all time.
    fn transform(
        &mut self,
        transform: &Transform<T>,
    ) -> Result<(), TransformError>;
}
