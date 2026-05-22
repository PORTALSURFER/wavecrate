//! Helpers for tracking waveform selection ranges and drag interactions.
//! This module keeps selection math pure and testable so the UI integration code can stay small.

/// Selection range geometry, fades, and gain helpers.
mod fade;
/// Decoded frame conversion helpers for normalized selection bounds.
mod frames;
mod range;
/// Selection drag state machine and edge-drag behavior.
mod state;

pub use fade::FadeParams;
pub(crate) use fade::fade_gain_at_position;
pub use frames::SampleFrameRange;
pub use range::SelectionRange;
pub use state::{SelectionEdge, SelectionState};

#[cfg(test)]
/// Selection behavior tests.
mod tests;
