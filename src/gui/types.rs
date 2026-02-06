//! Backend-neutral geometry and image buffer types.
#![allow(dead_code)]

/// 2D point in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    /// Construct a point from x/y coordinates.
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D vector in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Vector2 {
    pub x: f32,
    pub y: f32,
}

impl Vector2 {
    /// Construct a vector from x/y components.
    pub(crate) fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Axis-aligned rectangle in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    /// Construct a rectangle from minimum and maximum corners.
    pub(crate) fn from_min_max(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// Construct a rectangle from a minimum corner and size.
    pub(crate) fn from_min_size(min: Point, size: Vector2) -> Self {
        Self {
            min,
            max: Point::new(min.x + size.x, min.y + size.y),
        }
    }

    /// Rectangle width in logical coordinates.
    pub(crate) fn width(self) -> f32 {
        self.max.x - self.min.x
    }

    /// Rectangle height in logical coordinates.
    pub(crate) fn height(self) -> f32 {
        self.max.y - self.min.y
    }

    /// Return whether the point lies inside the rectangle bounds.
    pub(crate) fn contains(self, point: Point) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Return an inset rectangle.
    pub(crate) fn inset(self, amount: f32) -> Self {
        Self {
            min: Point::new(self.min.x + amount, self.min.y + amount),
            max: Point::new(self.max.x - amount, self.max.y - amount),
        }
    }
}

/// RGBA color in 8-bit per channel sRGB space.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Owned RGBA image buffer used by the GUI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImageRgba {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl ImageRgba {
    /// Create an image buffer from width/height and RGBA8 bytes.
    pub(crate) fn new(width: usize, height: usize, pixels: Vec<u8>) -> Option<Self> {
        if pixels.len() != width.saturating_mul(height).saturating_mul(4) {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels,
        })
    }
}
