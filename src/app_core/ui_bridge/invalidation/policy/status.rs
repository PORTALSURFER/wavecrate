use super::{InvalidationReason, InvalidationSource};
use crate::app_core::actions::{NativeOptionsAction, NativeUiAction};

pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(InvalidationSource, InvalidationReason)> {
    match action {
        NativeUiAction::HistoryAndUpdate(
            crate::app_core::actions::NativeHistoryUpdateAction::CheckForUpdates
            | crate::app_core::actions::NativeHistoryUpdateAction::OpenUpdateLink
            | crate::app_core::actions::NativeHistoryUpdateAction::InstallUpdate
            | crate::app_core::actions::NativeHistoryUpdateAction::DismissUpdate,
        )
        | NativeUiAction::Options(NativeOptionsAction::OpenOptionsMenu)
        | NativeUiAction::Options(NativeOptionsAction::CloseOptionsPanel)
        | NativeUiAction::Options(NativeOptionsAction::PickTrashFolder)
        | NativeUiAction::Options(NativeOptionsAction::OpenTrashFolder)
        | NativeUiAction::Options(NativeOptionsAction::SetInputMonitoringEnabled { .. })
        | NativeUiAction::Options(NativeOptionsAction::SetAdvanceAfterRatingEnabled { .. })
        | NativeUiAction::Options(NativeOptionsAction::SetDestructiveYoloMode { .. })
        | NativeUiAction::Options(NativeOptionsAction::SetInvertWaveformScroll { .. })
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::ConfirmPrompt,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CancelPrompt,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CancelProgress,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::SetPromptInput { .. },
        ) => Some((InvalidationSource::Status, InvalidationReason::StatusAction)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{NativeHistoryUpdateAction, NativePromptEditAction};

    #[test]
    fn options_and_prompt_actions_dirty_status_source() {
        let actions = [
            NativeUiAction::Options(NativeOptionsAction::OpenOptionsMenu),
            NativeUiAction::PromptsAndEdits(NativePromptEditAction::SetPromptInput {
                value: String::from("rename"),
            }),
        ];

        for action in actions {
            assert_eq!(
                classify_dirty_source(&action),
                Some((InvalidationSource::Status, InvalidationReason::StatusAction))
            );
        }
    }

    #[test]
    fn update_actions_dirty_status_source() {
        let action = NativeUiAction::HistoryAndUpdate(NativeHistoryUpdateAction::CheckForUpdates);

        assert_eq!(
            classify_dirty_source(&action),
            Some((InvalidationSource::Status, InvalidationReason::StatusAction))
        );
    }
}
