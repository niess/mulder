use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub};


#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Vec3 (pub [f64; 3]);

impl Vec3 {
    pub const ZERO: Self = Self([0.0; 3]);

    #[inline]
    pub fn clamp(self, a: f64, b: f64) -> Self {
        Self ([
            self.0[0].clamp(a, b),
            self.0[1].clamp(a, b),
            self.0[2].clamp(a, b),
        ])
    }

    #[inline]
    pub fn dot(rhs: &Self, lhs: &Self) -> f64 {
        rhs.0[0] * lhs.0[0] + rhs.0[1] * lhs.0[1] + rhs.0[2] * lhs.0[2]
    }

    #[inline]
    pub fn exp(&self) -> Self {
        Self ([
            self.0[0].exp(),
            self.0[1].exp(),
            self.0[2].exp(),
        ])
    }

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self ([x, y, z])
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let nrm = 1.0 / (self.0[0].powi(2) + self.0[1].powi(2) + self.0[2].powi(2)).sqrt();
        self * nrm
    }

    pub const fn splat(x: f64) -> Self {
        Self ([x; 3])
    }

    pub const fn x(&self) -> f64 {
        self.0[0]
    }

    pub const fn y(&self) -> f64 {
        self.0[1]
    }

    pub const fn z(&self) -> f64 {
        self.0[2]
    }
}

impl Add for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self ([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
        ])
    }
}

impl Add<f64> for Vec3 {
    type Output = Self;

    #[inline]
    fn add(self, rhs: f64) -> Self::Output {
        Self ([
            self.0[0] + rhs,
            self.0[1] + rhs,
            self.0[2] + rhs,
        ])
    }
}

impl Add<Vec3> for f64 {
    type Output = Vec3;

    #[inline]
    fn add(self, rhs: Vec3) -> Self::Output {
        Vec3 ([
            self + rhs.0[0],
            self + rhs.0[1],
            self + rhs.0[2],
        ])
    }
}

impl AddAssign for Vec3 {
    #[inline]
    fn add_assign(&mut self, rhs: Vec3) {
        self.0[0] += rhs.0[0];
        self.0[1] += rhs.0[1];
        self.0[2] += rhs.0[2];
    }
}

impl Div for Vec3 {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self::Output {
        Self ([
            self.0[0] / rhs.0[0],
            self.0[1] / rhs.0[1],
            self.0[2] / rhs.0[2],
        ])
    }
}

impl Div<f64> for Vec3 {
    type Output = Self;

    #[inline]
    fn div(self, rhs: f64) -> Self::Output {
        Self ([
            self.0[0] / rhs,
            self.0[1] / rhs,
            self.0[2] / rhs,
        ])
    }
}

impl DivAssign<f64> for Vec3 {
    #[inline]
    fn div_assign(&mut self, rhs: f64) {
        self.0[0] /= rhs;
        self.0[1] /= rhs;
        self.0[2] /= rhs;
    }
}

impl Mul for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self ([
            self.0[0] * rhs.0[0],
            self.0[1] * rhs.0[1],
            self.0[2] * rhs.0[2],
        ])
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: f64) -> Self::Output {
        Self ([
            self.0[0] * rhs,
            self.0[1] * rhs,
            self.0[2] * rhs,
        ])
    }
}

impl Mul<Vec3> for f64 {
    type Output = Vec3;

    #[inline]
    fn mul(self, rhs: Vec3) -> Self::Output {
        Vec3 ([
            self * rhs.0[0],
            self * rhs.0[1],
            self * rhs.0[2],
        ])
    }
}

impl MulAssign for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: Vec3) {
        self.0[0] *= rhs.0[0];
        self.0[1] *= rhs.0[1];
        self.0[2] *= rhs.0[2];
    }
}

impl MulAssign<f64> for Vec3 {
    #[inline]
    fn mul_assign(&mut self, rhs: f64) {
        self.0[0] *= rhs;
        self.0[1] *= rhs;
        self.0[2] *= rhs;
    }
}

impl Neg for Vec3 {
    type Output = Vec3;

    #[inline]
    fn neg(self) -> Self::Output {
        Vec3 ([
            -self.0[0],
            -self.0[1],
            -self.0[2],
        ])
    }
}

impl Sub for Vec3 {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self ([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
        ])
    }
}

impl Sub<Vec3> for f64 {
    type Output = Vec3;

    #[inline]
    fn sub(self, rhs: Vec3) -> Self::Output {
        Vec3 ([
            self - rhs.0[0],
            self - rhs.0[1],
            self - rhs.0[2],
        ])
    }
}
