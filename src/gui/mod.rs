//! Backend-agnostic GUI primitives used during the renderer migration.

/// Input event primitives shared by UI code.
pub mod input;
/// Native shell layout + scene model used by the experimental Vello backend.
pub(crate) mod native_shell;
/// Backend-neutral repaint signaling primitives used by runtimes and background jobs.
pub mod repaint;
/// Geometry and image buffer types shared by UI code.
pub mod types;
