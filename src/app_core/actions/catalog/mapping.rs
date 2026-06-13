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
    (@match SelectColumn { index }) => {
        NativeUiAction::Compatibility(
            crate::app_core::actions::NativeCompatibilityAction::SelectColumn { index: _ },
        )
    };
    (@match MoveColumn { delta }) => {
        NativeUiAction::Compatibility(
            crate::app_core::actions::NativeCompatibilityAction::MoveColumn { delta: _ },
        )
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
            | NativeUiAction::Compatibility(crate::app_core::actions::NativeCompatibilityAction::Undo)
    };
    (@match Redo {}) => {
        NativeUiAction::HistoryAndUpdate(crate::app_core::actions::NativeHistoryUpdateAction::Redo)
            | NativeUiAction::Compatibility(crate::app_core::actions::NativeCompatibilityAction::Redo)
    };
    (@match CheckForUpdates {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::CheckForUpdates,
        ) | NativeUiAction::Compatibility(crate::app_core::actions::NativeCompatibilityAction::CheckForUpdates)
    };
    (@match OpenUpdateLink {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::OpenUpdateLink,
        ) | NativeUiAction::Compatibility(crate::app_core::actions::NativeCompatibilityAction::OpenUpdateLink)
    };
    (@match InstallUpdate {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::InstallUpdate,
        ) | NativeUiAction::Compatibility(crate::app_core::actions::NativeCompatibilityAction::InstallUpdate)
    };
    (@match DismissUpdate {}) => {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::DismissUpdate,
        ) | NativeUiAction::Compatibility(crate::app_core::actions::NativeCompatibilityAction::DismissUpdate)
    };
    (@match OpenOptionsMenu {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenOptionsMenu)
    };
    (@match CloseOptionsPanel {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::CloseOptionsPanel)
    };
    (@match EditDefaultIdentifier {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::EditDefaultIdentifier)
    };
    (@match ShowOptionsOverview {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ShowOptionsOverview)
    };
    (@match PickTrashFolder {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::PickTrashFolder)
    };
    (@match OpenTrashFolder {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenTrashFolder)
    };
    (@match OpenAudioOutputHostPicker {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenAudioOutputHostPicker)
    };
    (@match OpenAudioOutputDevicePicker {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenAudioOutputDevicePicker)
    };
    (@match OpenAudioOutputSampleRatePicker {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenAudioOutputSampleRatePicker)
    };
    (@match OpenAudioInputHostPicker {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenAudioInputHostPicker)
    };
    (@match OpenAudioInputDevicePicker {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenAudioInputDevicePicker)
    };
    (@match OpenAudioInputSampleRatePicker {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::OpenAudioInputSampleRatePicker)
    };
    (@match SetAudioOutputHost { host_id }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAudioOutputHost { host_id: _ })
    };
    (@match SetAudioOutputDevice { device_name }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAudioOutputDevice { device_name: _ })
    };
    (@match SetAudioOutputSampleRate { sample_rate }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAudioOutputSampleRate { sample_rate: _ })
    };
    (@match SetAudioInputHost { host_id }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAudioInputHost { host_id: _ })
    };
    (@match SetAudioInputDevice { device_name }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAudioInputDevice { device_name: _ })
    };
    (@match SetAudioInputSampleRate { sample_rate }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAudioInputSampleRate { sample_rate: _ })
    };
    (@match SetInputMonitoringEnabled { enabled }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetInputMonitoringEnabled { enabled: _ })
    };
    (@match SetAdvanceAfterRatingEnabled { enabled }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetAdvanceAfterRatingEnabled { enabled: _ })
    };
    (@match SetDestructiveYoloMode { enabled }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetDestructiveYoloMode { enabled: _ })
    };
    (@match SetInvertWaveformScroll { enabled }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetInvertWaveformScroll { enabled: _ })
    };
    (@match SetVolume { value_milli }) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetVolume { value_milli: _ })
    };
    (@match CommitVolumeSetting {}) => {
        NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::CommitVolumeSetting)
    };
    (@match SeekWaveform { position_milli }) => {
        NativeUiAction::Compatibility(
            crate::app_core::actions::NativeCompatibilityAction::SeekWaveform {
                position_milli: _,
            },
        )
    };
    (@match SetWaveformCursor { position_milli }) => {
        NativeUiAction::Compatibility(
            crate::app_core::actions::NativeCompatibilityAction::SetWaveformCursor {
                position_milli: _,
            },
        )
    };
    (@match $kind:ident {}) => {
        NativeUiAction::$kind
    };
    (@match $kind:ident { $($field:ident),+ }) => {
        NativeUiAction::$kind { $($field: _,)+ .. }
    };
}

gui_action_rows!(build_action_mapping);
