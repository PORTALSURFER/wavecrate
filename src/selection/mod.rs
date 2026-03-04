//! Helpers for tracking waveform selection ranges and drag interactions.
//! This module keeps selection math pure and testable so the UI integration code can stay small.

/// Selection range geometry, fades, and gain helpers.
mod range;
/// Selection drag state machine and edge-drag behavior.
mod state;

pub(crate) use range::fade_gain_at_position;
pub use range::{FadeParams, SelectionRange};
pub use state::{SelectionEdge, SelectionState};

#[cfg(test)]
/// Selection behavior tests.
mod tests;
