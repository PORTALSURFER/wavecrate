//! Compatibility input adapters for older UI action payloads.
//!
//! New app-core code should construct current `UiAction` domain payloads
//! directly. This module owns the remaining supported legacy input shapes and
//! their upgrade rules so migration behavior stays separate from active action
//! ownership.

use super::{HistoryUpdateAction, UiAction, WaveformAction};
use serde::{Deserialize, Serialize};

/// Supported legacy action inputs retained for runtime and artifact readers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityAction {
    /// Older flat undo payload.
    Undo,
    /// Older flat redo payload.
    Redo,
    /// Older flat update-check payload.
    CheckForUpdates,
    /// Older flat update-link payload.
    OpenUpdateLink,
    /// Older flat update-install payload.
    InstallUpdate,
    /// Older flat update-dismiss payload.
    DismissUpdate,
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
            Self::Undo => UiAction::HistoryAndUpdate(HistoryUpdateAction::Undo),
            Self::Redo => UiAction::HistoryAndUpdate(HistoryUpdateAction::Redo),
            Self::CheckForUpdates => {
                UiAction::HistoryAndUpdate(HistoryUpdateAction::CheckForUpdates)
            }
            Self::OpenUpdateLink => UiAction::HistoryAndUpdate(HistoryUpdateAction::OpenUpdateLink),
            Self::InstallUpdate => UiAction::HistoryAndUpdate(HistoryUpdateAction::InstallUpdate),
            Self::DismissUpdate => UiAction::HistoryAndUpdate(HistoryUpdateAction::DismissUpdate),
            Self::SelectColumn { index } => UiAction::Compatibility(Self::SelectColumn { index }),
            Self::MoveColumn { delta } => UiAction::Compatibility(Self::MoveColumn { delta }),
            Self::SeekWaveform { position_milli } => {
                UiAction::Waveform(WaveformAction::SeekWaveformPrecise {
                    position_nanos: milli_to_nanos(position_milli),
                })
            }
            Self::SetWaveformCursor { position_milli } => {
                UiAction::Waveform(WaveformAction::SetWaveformCursorPrecise {
                    position_nanos: milli_to_nanos(position_milli),
                })
            }
        }
    }

    /// Return whether this compatibility input remains part of the durable
    /// compatibility contract.
    pub const fn policy(self) -> CompatibilityPolicy {
        match self {
            Self::Undo
            | Self::Redo
            | Self::CheckForUpdates
            | Self::OpenUpdateLink
            | Self::InstallUpdate
            | Self::DismissUpdate
            | Self::SeekWaveform { .. }
            | Self::SetWaveformCursor { .. } => CompatibilityPolicy::DurableUpgrade,
            Self::SelectColumn { .. } | Self::MoveColumn { .. } => CompatibilityPolicy::Review,
        }
    }
}

impl UiAction {
    /// Normalize retained compatibility payloads into current action shapes.
    pub fn upgrade_compatibility(self) -> Self {
        match self {
            UiAction::Compatibility(action) => action.upgrade(),
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
