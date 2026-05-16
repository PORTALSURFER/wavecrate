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
//! primitives in `wavecrate` and makes ownership boundaries enforceable in code review.

pub mod input {
    //! Shared key, pointer, and modifier tokens from `radiant`.
    //!
    //! The types are re-exported to avoid duplication of input vocabulary in
    //! application code.
    pub use radiant::gui::input::*;
}

pub mod layout_core {
    //! Generic slot-based layout primitives from `radiant`.
    pub use radiant::gui::layout_core::*;
}

pub mod text_layout {
    //! Generic text placement helpers from `radiant`.
    pub use radiant::gui::text_layout::*;
}

pub mod automation {
    //! Generic automation snapshot primitives from `radiant`.
    pub use radiant::gui::automation::*;
}

pub mod badge {
    //! Generic badge and pill primitives from `radiant`.
    pub use radiant::gui::badge::*;
}

pub mod chrome {
    //! Generic chrome and status-surface primitives from `radiant`.
    pub use radiant::gui::chrome::*;
}

pub mod feedback {
    //! Generic user-feedback surface primitives from `radiant`.
    pub use radiant::gui::feedback::*;
}

pub mod focus {
    //! Generic focus routing primitives from `radiant`.
    pub use radiant::gui::focus::*;
}

pub mod fingerprint {
    //! Generic stable fingerprint helpers from `radiant`.
    pub use radiant::gui::fingerprint::*;
}

pub mod form {
    //! Generic form and picker primitives from `radiant`.
    pub use radiant::gui::form::*;
}

pub mod frame {
    //! Frame feedback primitives from `radiant`.
    pub use radiant::gui::frame::*;
}

pub mod invalidation {
    //! Generic retained invalidation primitives from `radiant`.
    pub use radiant::gui::invalidation::*;
}

pub mod list {
    //! Generic list and virtualization primitives from `radiant`.
    //!
    //! Re-exports keep Wavecrate's large-list behavior on the framework-owned
    //! virtualization contract instead of duplicating list-window math locally.
    pub use radiant::gui::list::*;
}

pub mod paint {
    //! Backend-neutral paint primitives from `radiant`.
    pub use radiant::gui::paint::*;
}

pub mod panel {
    //! Generic panel and split-pane primitives from `radiant`.
    pub use radiant::gui::panel::*;
}

pub mod range {
    //! Normalized range and viewport projection primitives from `radiant`.
    //!
    //! Re-exported so Wavecrate-owned waveform and timeline surfaces use generic
    //! normalized coordinate math instead of duplicating projection helpers.
    pub use radiant::gui::range::*;
}

pub mod retained {
    //! Retained snapshot storage primitives from `radiant`.
    pub use radiant::gui::retained::*;
}

pub mod selection {
    //! Generic selection state primitives from `radiant`.
    pub use radiant::gui::selection::*;
}

pub mod shortcuts {
    //! Generic shortcut resolution primitives from `radiant`.
    pub use radiant::gui::shortcuts::*;
}

pub mod snapshot {
    //! Serializable visual snapshot primitives from `radiant`.
    pub use radiant::gui::snapshot::*;
}

pub mod repaint {
    //! Signals used to request UI updates from background work.
    //!
    //! Re-exports allow application subsystems to request deterministic paint
    //! invalidation without depending on runtime internals.
    pub use radiant::gui::repaint::*;
}

pub mod svg {
    //! SVG helpers from `radiant`.
    pub use radiant::gui::svg::*;
}

pub mod types {
    //! Light-weight value types used by UI declarations and render payloads.
    //!
    //! These types are intentionally constrained to data contracts (geometry,
    //! style primitives, IDs) and intentionally exclude behavior.
    pub use radiant::gui::types::*;
}

pub mod visualization {
    //! Generic visualization primitives from `radiant`.
    //!
    //! Re-exported so Wavecrate-owned waveform, timeline, and map surfaces consume
    //! framework-owned data contracts instead of compatibility aliases.
    pub use radiant::gui::visualization::*;
}
