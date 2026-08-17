//! A point in 3D space with position, orientation, timestamp, and reference frame.

use crate::{
    Localized, Transform, Transformable,
    errors::TransformError,
    geometry::{Quaternion, Vector3},
    time::{Stamp, TimePoint, Timestamp},
};

use alloc::string::String;
use approx::{AbsDiffEq, RelativeEq};

/// Represents a point in space with a position, orientation, timestamp, and its frame of reference.
///
/// The `Point` struct represents a single observation of data, at some given moment in time, with respect
/// to a specific reference frame. It encapsulates a 3D position using a `Vector3`, an orientation
/// using a `Quaternion`, a `Timestamp` to indicate when the point was recorded, and  a `String`
/// representing the coordinate reference frame its data is relative to.
///
/// A `Point` is a data record, not an invariant carrier: build it with
/// [`Point::new`] and read or write its fields freely. It is the reference
/// implementation of [`Transformable`] and [`Localized`]; the invariants that
/// matter live on the [`Transform`] being applied, not here.
///
/// With the optional `serde` feature, this type implements `Serialize` and
/// `Deserialize` (the docs.rs listing cannot banner derive-generated impls).
///
/// # Examples
///
/// ```
/// use transforms::{
///     geometry::{Point, Quaternion, Vector3},
///     time::Timestamp,
/// };
///
/// let point: Point = Point::new(
///     Vector3::new(1.0, 2.0, 3.0),
///     Quaternion::identity(),
///     Timestamp::zero(),
///     "a",
/// );
///
/// assert_eq!(point.position.x, 1.0);
/// assert_eq!(point.orientation.w, 1.0);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub struct Point<T = Timestamp>
where
    T: TimePoint,
{
    /// The 3D position of the point.
    pub position: Vector3,
    /// The orientation of the point.
    pub orientation: Quaternion,
    /// The time at which the point was recorded.
    pub timestamp: T,
    /// The reference frame the point's data is relative to.
    pub frame: String,
}

impl<T> Point<T>
where
    T: TimePoint,
{
    /// Builds a point observed in `frame` at `timestamp`.
    ///
    /// # Examples
    ///
    /// ```
    /// use transforms::{
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
    /// assert_eq!(point.frame, "camera");
    /// ```
    #[must_use]
    pub fn new(
        position: Vector3,
        orientation: Quaternion,
        timestamp: T,
        frame: &str,
    ) -> Self {
        Self {
            position,
            orientation,
            timestamp,
            frame: frame.into(),
        }
    }
}

/// The `Transformable` trait defines an interface for objects that can be transformed
/// using a `Transform`. Implementors of this trait can apply a transformation to
/// themselves, modifying their position and orientation.
///
/// # Examples
///
/// ```
/// use transforms::{
///     Transform, Transformable,
///     geometry::{Point, Quaternion, Vector3},
///     time::{Stamp, Timestamp},
/// };
///
/// let mut point: Point = Point::new(
///     Vector3::new(1.0, 2.0, 3.0),
///     Quaternion::identity(),
///     Timestamp::zero(),
///     "b",
/// );
///
/// let transform: Transform = Transform::new(
///     "a",
///     "b",
///     Vector3::new(2.0, 0.0, 0.0),
///     Quaternion::identity(),
///     Stamp::At(Timestamp::zero()),
/// )
/// .unwrap();
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
    /// Applies a transformation to the `Point`, updating its position, orientation, and frame.
    ///
    /// The transform's geometry is applied as given: a `Transform` is valid by
    /// construction, so there is nothing left to check here beyond the frame
    /// and the time.
    ///
    /// # Errors
    ///
    /// Returns a [`TransformError`] if the point's frame does not match the transform's child
    /// frame, or if the timestamps do not match. Static transforms (carrying
    /// `Stamp::Static`, e.g. built with `Transform::static_between`) are
    /// valid for all time and apply to a point of any timestamp.
    fn transform(
        &mut self,
        transform: &Transform<T>,
    ) -> Result<(), TransformError> {
        if self.frame != transform.child() {
            return Err(TransformError::IncompatibleFrames {
                expected: transform.child().into(),
                found: self.frame.clone(),
            });
        }
        match transform.timestamp() {
            // A static transform is valid for all time and applies to a
            // point of any timestamp.
            Stamp::Static => {}
            Stamp::At(t) if t == self.timestamp => {}
            Stamp::At(t) => {
                return Err(TransformError::TimestampMismatch {
                    lhs: self.timestamp.as_seconds_lossy(),
                    rhs: t.as_seconds_lossy(),
                });
            }
        }
        self.position = transform.rotation().rotate_vector(self.position) + transform.translation();
        self.orientation = transform.rotation() * self.orientation;
        self.frame = transform.parent().into();
        Ok(())
    }
}

/// The `Localized` trait provides frame and timestamp introspection for a `Point`,
/// enabling automatic transform lookup via
/// [`Registry::get_transform_for`](crate::Registry::get_transform_for).
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "std")]
/// use core::time::Duration;
/// use transforms::{
///     Registry, Transformable,
///     geometry::{Point, Quaternion, Transform, Vector3},
///     time::{Stamp, Timestamp},
/// };
///
/// # #[cfg(feature = "std")]
/// let mut registry = Registry::with_max_age(Duration::from_secs(10));
/// # #[cfg(not(feature = "std"))]
/// # let mut registry = Registry::new();
/// # #[cfg(feature = "std")]
/// let t = Timestamp::now();
/// # #[cfg(not(feature = "std"))]
/// # let t = Timestamp::zero();
///
/// registry
///     .add_transform(
///         Transform::new(
///             "map",
///             "camera",
///             Vector3::new(1.0, 0.0, 0.0),
///             Quaternion::identity(),
///             Stamp::At(t),
///         )
///         .unwrap(),
///     )
///     .unwrap();
///
/// let mut point = Point::new(
///     Vector3::new(1.0, 0.0, 0.0),
///     Quaternion::identity(),
///     t,
///     "camera",
/// );
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

impl<T> AbsDiffEq for Point<T>
where
    T: TimePoint,
{
    type Epsilon = f64;

    fn default_epsilon() -> Self::Epsilon {
        f64::EPSILON
    }

    /// Compares position and orientation within `epsilon`; frame and
    /// timestamp must match exactly.
    fn abs_diff_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
    ) -> bool {
        self.position.abs_diff_eq(&other.position, epsilon)
            && self.orientation.abs_diff_eq(&other.orientation, epsilon)
            && self.timestamp == other.timestamp
            && self.frame == other.frame
    }
}

impl<T> RelativeEq for Point<T>
where
    T: TimePoint,
{
    fn default_max_relative() -> Self::Epsilon {
        f64::EPSILON
    }

    /// Compares position and orientation with relative tolerance; frame and
    /// timestamp must match exactly.
    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.position
            .relative_eq(&other.position, epsilon, max_relative)
            && self
                .orientation
                .relative_eq(&other.orientation, epsilon, max_relative)
            && self.timestamp == other.timestamp
            && self.frame == other.frame
    }
}

#[cfg(test)]
mod tests;
