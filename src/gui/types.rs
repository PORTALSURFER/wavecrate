//! Backend-neutral geometry and image buffer types.
#![allow(dead_code)]

/// 2D point in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Point {
    pub x: f32,
    pub y: f32,
}

/// 2D vector in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Vector2 {
    pub x: f32,
    pub y: f32,
}

/// Axis-aligned rectangle in logical UI coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub min: Point,
    pub max: Point,
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
