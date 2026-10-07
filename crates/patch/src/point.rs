//! A point in three dimensions, which is also what a displacement is.
//!
//! One type for both is not a shortcut. A refinement level stores a *displacement* against the
//! root net, and the reason an empty level contributes exactly nothing is that the interpolation is
//! linear in the control points — which is only visibly true when the thing being summed is
//! described by the same type as the thing being added to.

use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// A point in three dimensions. The same type serves as a position and as a displacement.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Euclidean length.
    pub fn length(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }

    /// Distance to `other`.
    pub fn distance(self, other: Self) -> f64 {
        (self - other).length()
    }

    /// The angle between `self` and `other` in radians, in `[0, pi]`.
    ///
    /// Two vectors of zero length have no angle; that case returns `f64::NAN` rather than a
    /// number a caller could mistake for a measurement.
    pub fn angle(self, other: Self) -> f64 {
        let denominator = self.length() * other.length();
        if denominator == 0.0 {
            return f64::NAN;
        }
        (self.dot(other) / denominator).clamp(-1.0, 1.0).acos()
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
}

impl Add for Point3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl AddAssign for Point3 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl Sub for Point3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl SubAssign for Point3 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl Neg for Point3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<f64> for Point3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Mul<Point3> for f64 {
    type Output = Point3;
    fn mul(self, rhs: Point3) -> Point3 {
        rhs * self
    }
}

/// Division exists because finite differences need it: a derivative is a difference of two
/// surface points over a parameter step.
impl Div<f64> for Point3 {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        Self::new(self.x / rhs, self.y / rhs, self.z / rhs)
    }
}
