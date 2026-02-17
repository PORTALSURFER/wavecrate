//! Backend-agnostic GUI primitives re-exported from the standalone `radiant` crate.
//!
//! `src/gui` is intentionally minimal: it exposes the host-agnostic GUI API surface
//! from `radiant` and avoids implementing widgets, layout policies, event semantics,
//! or rendering orchestration.

pub mod input {
    //! Shared key, pointer, and modifier event values.
    pub use radiant::gui::input::*;
}

pub mod repaint {
    //! Signals used to request UI updates from background work.
    pub use radiant::gui::repaint::*;
}

pub mod types {
    //! Light-weight value types used by UI declarations and render payloads.
    pub use radiant::gui::types::*;
}
