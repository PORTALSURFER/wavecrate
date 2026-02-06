//! Backend-neutral paint primitives emitted by the native shell.

use crate::gui::types::{Point, Rect, Rgba8};

/// Filled rectangle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillRect {
    pub rect: Rect,
    pub color: Rgba8,
}

/// Filled circle draw primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FillCircle {
    pub center: Point,
    pub radius: f32,
    pub color: Rgba8,
}

/// Horizontal alignment strategy for text runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextAlign {
    Left,
    Center,
    Right,
}

/// Single-line text primitive emitted by the shell paint pass.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TextRun {
    /// Text content.
    pub text: String,
    /// Top-left anchor point for the run.
    pub position: Point,
    /// Font size in logical pixels-per-em.
    pub font_size: f32,
    /// Text color.
    pub color: Rgba8,
    /// Optional clipping width.
    pub max_width: Option<f32>,
    /// Horizontal alignment inside `max_width`.
    pub align: TextAlign,
}

/// Backend-neutral scene primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Primitive {
    Rect(FillRect),
    Circle(FillCircle),
}

/// Full frame emitted by the retained shell render pipeline.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NativeViewFrame {
    /// Root clear color.
    pub clear_color: Rgba8,
    /// Shape primitives.
    pub primitives: Vec<Primitive>,
    /// Text primitives.
    pub text_runs: Vec<TextRun>,
}
