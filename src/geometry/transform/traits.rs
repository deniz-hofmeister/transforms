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
/// # Contract
///
/// An implementation owes the rigid-body map, in this order: every bound
/// position `p` becomes
/// `transform.rotation().rotate_vector(p) + transform.translation()` —
/// rotate first, then translate — and every orientation `q` becomes
/// `transform.rotation() * q`, the transform's rotation on the left. A
/// free vector — a velocity, a surface normal — takes the rotation only,
/// and owes no translation. Where the object carries them, as
/// [`Point`](crate::geometry::Point) does, its frame becomes the
/// transform's parent frame; timestamps are checked, never rewritten.
/// The reversed variants compile, and each has a blind spot that keeps
/// weak tests green: translating before rotating agrees with the
/// contract until a real rotation meets a non-zero translation, and
/// `q * transform.rotation()` agrees until the object's own orientation
/// is non-identity and does not commute with the transform's. Beyond the
/// blind spot both produce a silent wrong answer, never a loud failure.
/// `Point`'s implementation is the reference, and the suite pins both
/// orders.
///
/// # Precondition
///
/// An implementation applies the transform's geometry as given; it checks
/// frames and time, not numbers. A [`Transform`] built through its
/// constructors or read through its `Deserialize` impl was checked there —
/// both reject non-finite components and non-unit rotations. One *derived*
/// from valid transforms was not: `*`, [`Transform::inverse`],
/// [`Transform::interpolate`] and registry lookups deliberately skip the
/// re-check, and composing operands at the edge of the tolerance walks past
/// it. Applying such a transform deserves a [`Transform::validate`] call
/// first: a rotation whose norm is 1.01 scales everything it touches by 2%
/// and reports success.
///
/// # Errors
///
/// Returns `TransformError` if:
/// - The frames are incompatible (transform's child frame doesn't match the object's frame)
/// - The timestamps don't match — except for static transforms (carrying
///   `Stamp::Static`, e.g. built with `Transform::static_between`), which
///   are valid for all time
/// - Other transform-specific errors occur
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
/// point
///     .transform(&transform)
///     .expect("failed to transform point");
/// ```
pub trait Transformable<T = Timestamp>
where
    T: TimePoint,
{
    /// Applies a transform to this object, modifying it in place.
    ///
    /// What "applies" must compute — rotate, then translate; the
    /// transform's rotation on the left of the orientation composition —
    /// is pinned by the trait-level Contract section.
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
