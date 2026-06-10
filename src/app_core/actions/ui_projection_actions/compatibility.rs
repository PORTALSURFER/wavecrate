//! Compatibility input adapters for older UI action payloads.
//!
//! New app-core code should construct current `UiAction` domain payloads
//! directly. This module owns the remaining supported legacy input shapes and
//! their upgrade rules so migration behavior stays separate from active action
//! ownership.

use super::UiAction;
use serde::{Deserialize, Serialize};

/// Supported legacy action inputs retained for runtime and artifact readers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityAction {
    /// Older triage-column selection payload.
    SelectColumn {
        /// Target column index in the visible triage column set.
        index: usize,
    },
    /// Older triage-column focus payload.
    MoveColumn {
        /// Signed column delta (`-1` for left, `+1` for right).
        delta: i8,
    },
    /// Older waveform seek payload using normalized milli-units.
    SeekWaveform {
        /// Normalized milli target position (`0..=1000`).
        position_milli: u16,
    },
    /// Older waveform cursor payload using normalized milli-units.
    SetWaveformCursor {
        /// Normalized milli cursor position (`0..=1000`).
        position_milli: u16,
    },
}

impl CompatibilityAction {
    /// Upgrade one retained compatibility input to the current action contract.
    pub fn upgrade(self) -> UiAction {
        match self {
            Self::SelectColumn { index } => UiAction::SelectColumn { index },
            Self::MoveColumn { delta } => UiAction::MoveColumn { delta },
            Self::SeekWaveform { position_milli } => UiAction::SeekWaveformPrecise {
                position_nanos: milli_to_nanos(position_milli),
            },
            Self::SetWaveformCursor { position_milli } => UiAction::SetWaveformCursorPrecise {
                position_nanos: milli_to_nanos(position_milli),
            },
        }
    }

    /// Return whether this compatibility input remains part of the durable
    /// compatibility contract.
    pub const fn policy(self) -> CompatibilityPolicy {
        match self {
            Self::SelectColumn { .. } | Self::MoveColumn { .. } => CompatibilityPolicy::Review,
            Self::SeekWaveform { .. } | Self::SetWaveformCursor { .. } => {
                CompatibilityPolicy::DurableUpgrade
            }
        }
    }
}

impl UiAction {
    /// Normalize retained compatibility payloads into current action shapes.
    pub fn upgrade_compatibility(self) -> Self {
        match self {
            UiAction::SelectColumn { index } => {
                CompatibilityAction::SelectColumn { index }.upgrade()
            }
            UiAction::MoveColumn { delta } => CompatibilityAction::MoveColumn { delta }.upgrade(),
            UiAction::SeekWaveform { position_milli } => {
                CompatibilityAction::SeekWaveform { position_milli }.upgrade()
            }
            UiAction::SetWaveformCursor { position_milli } => {
                CompatibilityAction::SetWaveformCursor { position_milli }.upgrade()
            }
            action => action,
        }
    }
}

/// Compatibility support policy for a retained legacy input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompatibilityPolicy {
    /// Keep parsing for now, but move callers to a domain-owned replacement.
    Review,
    /// Keep parsing and upgrade to a current action at the adapter boundary.
    DurableUpgrade,
}

/// Upgrade one optional compatibility action.
pub fn upgrade_compatibility_action(action: CompatibilityAction) -> UiAction {
    action.upgrade()
}

pub(crate) const fn milli_to_nanos(position_milli: u16) -> u32 {
    let clamped_milli = if position_milli > 1000 {
        1000
    } else {
        position_milli
    };
    (clamped_milli as u32) * 1_000_000
}

#[cfg(test)]
mod tests;
