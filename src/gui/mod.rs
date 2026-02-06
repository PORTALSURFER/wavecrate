//! Backend-agnostic GUI primitives used during the renderer migration.

/// Input event primitives shared by UI code.
pub mod input;
/// Backend-neutral repaint signaling primitives used by runtimes and background jobs.
pub mod repaint;
/// Geometry and image buffer types shared by UI code.
pub mod types;
