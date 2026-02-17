//! Backend-agnostic GUI primitives re-exported from the standalone `radiant` crate.
//!
//! Architectural boundary:
//!
//! * `src/gui` exposes `radiant` API types only (inputs, layout-independent types,
//!   repaint signals).
//! * it performs no widget construction, state transitions, layout decisions, or hit
//!   testing.
//! * it performs no input normalization or propagation policy.
//! * it performs no rendering orchestration.
//!
//! Keeping these modules as pure re-exports prevents accidental duplication of GUI
//! primitives in `sempal` and makes ownership boundaries enforceable in code review.

pub mod input {
    //! Shared key, pointer, and modifier tokens from `radiant`.
    //!
    //! The types are re-exported to avoid duplication of input vocabulary in
    //! application code.
    pub use radiant::gui::input::*;
}

pub mod repaint {
    //! Signals used to request UI updates from background work.
    //!
    //! Re-exports allow application subsystems to request deterministic paint
    //! invalidation without depending on runtime internals.
    pub use radiant::gui::repaint::*;
}

pub mod types {
    //! Light-weight value types used by UI declarations and render payloads.
    //!
    //! These types are intentionally constrained to data contracts (geometry,
    //! style primitives, IDs) and intentionally exclude behavior.
    pub use radiant::gui::types::*;
}
