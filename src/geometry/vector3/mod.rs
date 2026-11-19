//! Three plain `f64` components.

/// Three `f64` components: a translation, or a point's coordinates.
///
/// The fields are public and carry no invariant. Finiteness is checked
/// where a vector enters a [`Transform`](crate::Transform), not here, and
/// there are no arithmetic operators: the one operation this crate applies
/// to a point is [`Rotation::rotate`](crate::Rotation::rotate), and the
/// [`Transform`](crate::Transform) documentation shows how a transform is
/// applied with it.
///
/// Equality is exact IEEE 754 comparison: `NaN` never compares equal and
/// `0.0 == -0.0`. With `serde`, a vector decodes unchecked.
///
/// # Examples
///
/// ```
/// use transforms::Vector3;
///
/// let v = Vector3::new(1.0, 2.0, 3.0);
/// assert_eq!(v.y, 2.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Vector3 {
    /// The `x` component.
    pub x: f64,
    /// The `y` component.
    pub y: f64,
    /// The `z` component.
    pub z: f64,
}

impl Vector3 {
    /// The origin: the translation of the identity.
    pub(crate) const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    /// Creates a vector from its components.
    #[must_use]
    pub const fn new(
        x: f64,
        y: f64,
        z: f64,
    ) -> Self {
        Self { x, y, z }
    }

    /// Whether all three components are finite.
    pub(crate) fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

#[cfg(test)]
mod tests;
