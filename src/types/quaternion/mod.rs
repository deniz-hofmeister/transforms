use crate::types::Vector3;
use core::ops::{Add, Div, Mul, Sub};
pub mod error;
pub use error::QuaternionError;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    const EPSILON: f64 = 1e-9;

    #[inline]
    pub fn conjugate(self) -> Quaternion {
        Quaternion {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    #[inline]
    pub fn normalize(self) -> Result<Quaternion, QuaternionError> {
        let norm = self.norm();
        if norm < Self::EPSILON {
            return Err(QuaternionError::ZeroLengthNormalization);
        }
        Ok(self.scale(1.0 / norm))
    }

    #[inline]
    pub fn norm(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    #[inline]
    pub fn norm_squared(self) -> f64 {
        self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline]
    pub fn scale(
        self,
        factor: f64,
    ) -> Quaternion {
        Quaternion {
            w: self.w * factor,
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    #[inline]
    pub fn rotate_vector(
        self,
        v: Vector3,
    ) -> Vector3 {
        let q_vec = Quaternion {
            w: 0.0,
            x: v.x,
            y: v.y,
            z: v.z,
        };
        let q_res = self.mul(q_vec).mul(self.conjugate());
        Vector3 {
            x: q_res.x,
            y: q_res.y,
            z: q_res.z,
        }
    }

    #[inline]
    pub fn slerp(
        self,
        other: Quaternion,
        t: f64,
    ) -> Quaternion {
        let dot = self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z;

        let dot = dot.clamp(-1.0, 1.0);
        let theta = dot.acos();

        if theta.abs() < Self::EPSILON {
            return self.scale(1.0 - t) + other.scale(t);
        }

        let sin_theta = theta.sin();
        let scale_self = ((1.0 - t) * theta).sin() / sin_theta;
        let scale_other = (t * theta).sin() / sin_theta;

        self.scale(scale_self) + other.scale(scale_other)
    }
}

impl Add for Quaternion {
    type Output = Quaternion;

    #[inline]
    fn add(
        self,
        other: Quaternion,
    ) -> Quaternion {
        Quaternion {
            w: self.w + other.w,
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Quaternion {
    type Output = Quaternion;

    #[inline]
    fn sub(
        self,
        other: Quaternion,
    ) -> Quaternion {
        Quaternion {
            w: self.w - other.w,
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Mul for Quaternion {
    type Output = Quaternion;

    #[inline]
    fn mul(
        self,
        other: Quaternion,
    ) -> Quaternion {
        Quaternion {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }
}

impl Div for Quaternion {
    type Output = Result<Quaternion, QuaternionError>;

    #[inline]
    fn div(
        self,
        other: Quaternion,
    ) -> Result<Quaternion, QuaternionError> {
        let norm_sq = other.norm_squared();
        if norm_sq < Quaternion::EPSILON {
            return Err(QuaternionError::DivisionByZero);
        }
        Ok(self.mul(other.conjugate()).scale(1.0 / norm_sq))
    }
}

#[cfg(test)]
mod tests;
