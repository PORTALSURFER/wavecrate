use core::ops::{Add, AddAssign, Neg, Sub, SubAssign};

/// Geometry primitive used for UI-independent 2D points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPoint {
    /// X coordinate in UI pixels.
    pub x: f32,
    /// Y coordinate in UI pixels.
    pub y: f32,
}

impl UiPoint {
    /// Zero point.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Construct a point from coordinates.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Convert to a tuple.
    pub const fn to_tuple(self) -> (f32, f32) {
        (self.x, self.y)
    }

    /// Squared length from origin.
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Squared distance to another point.
    pub fn distance_sq(self, other: Self) -> f32 {
        (self - other).length_sq()
    }
}

impl Default for UiPoint {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<(f32, f32)> for UiPoint {
    fn from((x, y): (f32, f32)) -> Self {
        Self::new(x, y)
    }
}

impl From<UiPoint> for (f32, f32) {
    fn from(point: UiPoint) -> Self {
        point.to_tuple()
    }
}

impl Add for UiPoint {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign for UiPoint {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for UiPoint {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign for UiPoint {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Neg for UiPoint {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}
