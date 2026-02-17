//! Backend-agnostic GUI primitives re-exported from the standalone `radiant` crate.
//!
//! `src/gui` is intentionally minimal: it exposes the host-agnostic GUI API surface
//! from `radiant` and avoids implementing widgets, layout policies, event semantics,
//! or rendering orchestration.

/// Input event primitives shared by UI code.
pub use radiant::gui::input;
/// Backend-neutral repaint signaling primitives used by runtimes and background jobs.
pub use radiant::gui::repaint;
/// Geometry and image buffer types shared by UI code.
pub use radiant::gui::types;
