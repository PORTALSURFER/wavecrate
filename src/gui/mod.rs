//! Backend-agnostic GUI primitives re-exported from the standalone `radiant` crate.

/// Input event primitives shared by UI code.
pub use radiant::gui::input;
/// Backend-neutral repaint signaling primitives used by runtimes and background jobs.
pub use radiant::gui::repaint;
/// Geometry and image buffer types shared by UI code.
pub use radiant::gui::types;
