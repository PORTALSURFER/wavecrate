//! GUI action kind/sample mapping derived from the shared catalog rows.
//!
//! This module owns payload-to-kind matching and representative sample payload
//! generation. It must not redefine catalog metadata or policy tables.

use super::super::NativeUiAction;
use super::GuiActionKind;
use super::data::gui_action_rows;

macro_rules! build_action_mapping {
    ($(
        $kind:ident $pattern:tt => {
            id: $id:literal,
            surface: $surface:ident,
            effect: $effect:ident,
            coverage: [$($coverage:ident),+ $(,)?],
            fixtures: [$($fixture:literal),* $(,)?],
            sample: $sample:expr
        }
    ),+ $(,)?) => {
        /// Return the payload-free kind for one concrete UI action.
        pub fn action_kind(action: &NativeUiAction) -> GuiActionKind {
            match action {
                $(build_action_mapping!(@match $kind $pattern) => GuiActionKind::$kind,)+
            }
        }

        /// Return a representative action payload for the provided kind.
        pub fn representative_action_for_kind(kind: GuiActionKind) -> NativeUiAction {
            match kind {
                $(GuiActionKind::$kind => $sample,)+
            }
        }
    };
    (@match ToggleTransport {}) => {
        NativeUiAction::Transport(crate::app_core::actions::NativeTransportAction::ToggleTransport)
    };
    (@match PlayCompareAnchor {}) => {
        NativeUiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayCompareAnchor)
    };
    (@match PlayFromStart {}) => {
        NativeUiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromStart)
    };
    (@match PlayFromCurrentPlayhead {}) => {
        NativeUiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromCurrentPlayhead)
    };
    (@match PlayFromWaveformCursor {}) => {
        NativeUiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromWaveformCursor)
    };
    (@match PlayWaveformAtPrecise { position_nanos }) => {
        NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayWaveformAtPrecise {
                position_nanos: _,
            },
        )
    };
    (@match HandleEscape {}) => {
        NativeUiAction::Transport(crate::app_core::actions::NativeTransportAction::HandleEscape)
    };
    (@match Undo {}) => {
        NativeUiAction::HistoryAndUpdate(crate::app_core::actions::NativeHistoryUpdateAction::Undo)
            | NativeUiAction::Undo
    };
    (@match Redo {}) => {
        NativeUiAction::HistoryAndUpdate(crate::app_core::actions::NativeHistoryUpdateAction::Redo)
            | NativeUiAction::Redo
    };
    (@match CheckForUpdates {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::CheckForUpdates,
        ) | NativeUiAction::CheckForUpdates
    };
    (@match OpenUpdateLink {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::OpenUpdateLink,
        ) | NativeUiAction::OpenUpdateLink
    };
    (@match InstallUpdate {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::InstallUpdate,
        ) | NativeUiAction::InstallUpdate
    };
    (@match DismissUpdate {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::DismissUpdate,
        ) | NativeUiAction::DismissUpdate
    };
    (@match $kind:ident {}) => {
        NativeUiAction::$kind
    };
    (@match $kind:ident { $($field:ident),+ }) => {
        NativeUiAction::$kind { $($field: _,)+ .. }
    };
}

gui_action_rows!(build_action_mapping);
