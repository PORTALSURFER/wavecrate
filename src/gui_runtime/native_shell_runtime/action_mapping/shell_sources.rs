use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::SelectColumn { index } => Ok(UiAction::SelectColumn { index }),
        runtime_contract::UiAction::MoveColumn { delta } => Ok(UiAction::MoveColumn { delta }),
        runtime_contract::UiAction::ToggleTransport => Ok(UiAction::ToggleTransport),
        runtime_contract::UiAction::PlayCompareAnchor => Ok(UiAction::PlayCompareAnchor),
        runtime_contract::UiAction::PlayFromStart => Ok(UiAction::PlayFromStart),
        runtime_contract::UiAction::PlayFromCurrentPlayhead => {
            Ok(UiAction::PlayFromCurrentPlayhead)
        }
        runtime_contract::UiAction::PlayFromWaveformCursor => Ok(UiAction::PlayFromWaveformCursor),
        runtime_contract::UiAction::PlayWaveformAtPrecise { position_nanos } => {
            Ok(UiAction::PlayWaveformAtPrecise { position_nanos })
        }
        runtime_contract::UiAction::HandleEscape => Ok(UiAction::HandleEscape),
        runtime_contract::UiAction::FocusBrowserPanel => Ok(UiAction::FocusBrowserPanel),
        runtime_contract::UiAction::FocusSourcesPanel => Ok(UiAction::FocusSourcesPanel),
        runtime_contract::UiAction::FocusWaveformPanel => Ok(UiAction::FocusWaveformPanel),
        runtime_contract::UiAction::FocusFolderPanel => Ok(UiAction::FocusFolderPanel),
        runtime_contract::UiAction::FocusLoadedContentInList => {
            Ok(UiAction::FocusLoadedSampleInBrowser)
        }
        runtime_contract::UiAction::FocusBrowserSearch => Ok(UiAction::FocusBrowserSearch),
        runtime_contract::UiAction::BlurBrowserSearch => Ok(UiAction::BlurBrowserSearch),
        runtime_contract::UiAction::OpenAddSourceDialog => Ok(UiAction::OpenAddSourceDialog),
        runtime_contract::UiAction::OpenOptionsMenu => Ok(UiAction::OpenOptionsMenu),
        runtime_contract::UiAction::CloseOptionsPanel => Ok(UiAction::CloseOptionsPanel),
        runtime_contract::UiAction::PickTrashFolder => Ok(UiAction::PickTrashFolder),
        runtime_contract::UiAction::OpenTrashFolder => Ok(UiAction::OpenTrashFolder),
        runtime_contract::UiAction::EditDefaultIdentifier => Ok(UiAction::EditDefaultIdentifier),
        runtime_contract::UiAction::ShowOptionsOverview => Ok(UiAction::ShowOptionsOverview),
        runtime_contract::UiAction::OpenPrimaryGroupPicker => {
            Ok(UiAction::OpenAudioOutputHostPicker)
        }
        runtime_contract::UiAction::OpenPrimaryItemPicker => {
            Ok(UiAction::OpenAudioOutputDevicePicker)
        }
        runtime_contract::UiAction::OpenPrimaryNumberPicker => {
            Ok(UiAction::OpenAudioOutputSampleRatePicker)
        }
        runtime_contract::UiAction::OpenSecondaryGroupPicker => {
            Ok(UiAction::OpenAudioInputHostPicker)
        }
        runtime_contract::UiAction::OpenSecondaryItemPicker => {
            Ok(UiAction::OpenAudioInputDevicePicker)
        }
        runtime_contract::UiAction::OpenSecondaryNumberPicker => {
            Ok(UiAction::OpenAudioInputSampleRatePicker)
        }
        runtime_contract::UiAction::SetPrimaryGroup { group_id } => {
            Ok(UiAction::SetAudioOutputHost { host_id: group_id })
        }
        runtime_contract::UiAction::SetPrimaryItem { item_name } => {
            Ok(UiAction::SetAudioOutputDevice {
                device_name: item_name,
            })
        }
        runtime_contract::UiAction::SetPrimaryNumber { value } => {
            Ok(UiAction::SetAudioOutputSampleRate { sample_rate: value })
        }
        runtime_contract::UiAction::SetSecondaryGroup { group_id } => {
            Ok(UiAction::SetAudioInputHost { host_id: group_id })
        }
        runtime_contract::UiAction::SetSecondaryItem { item_name } => {
            Ok(UiAction::SetAudioInputDevice {
                device_name: item_name,
            })
        }
        runtime_contract::UiAction::SetSecondaryNumber { value } => {
            Ok(UiAction::SetAudioInputSampleRate { sample_rate: value })
        }
        runtime_contract::UiAction::FocusFolderSearch => Ok(UiAction::FocusFolderSearch),
        runtime_contract::UiAction::SetFolderSearch { query } => {
            Ok(UiAction::SetFolderSearch { query })
        }
        runtime_contract::UiAction::ToggleShowAllFolders => Ok(UiAction::ToggleShowAllFolders),
        runtime_contract::UiAction::ToggleFolderFlattenedView => {
            Ok(UiAction::ToggleFolderFlattenedView)
        }
        runtime_contract::UiAction::FocusSourceRow { index } => {
            Ok(UiAction::FocusSourceRow { index })
        }
        runtime_contract::UiAction::SelectSourceRow { index } => {
            Ok(UiAction::SelectSourceRow { index })
        }
        runtime_contract::UiAction::MoveSourceFocus { delta } => {
            Ok(UiAction::MoveSourceFocus { delta })
        }
        runtime_contract::UiAction::ReloadFocusedSourceRow => Ok(UiAction::ReloadFocusedSourceRow),
        runtime_contract::UiAction::HardSyncFocusedSourceRow => {
            Ok(UiAction::HardSyncFocusedSourceRow)
        }
        runtime_contract::UiAction::OpenFocusedSourceFolder => {
            Ok(UiAction::OpenFocusedSourceFolder)
        }
        runtime_contract::UiAction::RemoveFocusedSourceRow => Ok(UiAction::RemoveFocusedSourceRow),
        runtime_contract::UiAction::ReloadSourceRow { index } => {
            Ok(UiAction::ReloadSourceRow { index })
        }
        runtime_contract::UiAction::HardSyncSourceRow { index } => {
            Ok(UiAction::HardSyncSourceRow { index })
        }
        runtime_contract::UiAction::OpenSourceFolderRow { index } => {
            Ok(UiAction::OpenSourceFolderRow { index })
        }
        runtime_contract::UiAction::RemoveSourceRow { index } => {
            Ok(UiAction::RemoveSourceRow { index })
        }
        runtime_contract::UiAction::FocusFolderRow { index } => {
            Ok(UiAction::FocusFolderRow { index })
        }
        runtime_contract::UiAction::ActivateFolderRow { index } => {
            Ok(UiAction::ActivateFolderRow { index })
        }
        runtime_contract::UiAction::ToggleFolderRowExpanded { index } => {
            Ok(UiAction::ToggleFolderRowExpanded { index })
        }
        runtime_contract::UiAction::ExpandFocusedFolder => Ok(UiAction::ExpandFocusedFolder),
        runtime_contract::UiAction::CollapseFocusedFolder => Ok(UiAction::CollapseFocusedFolder),
        runtime_contract::UiAction::ToggleFocusedFolderSelection => {
            Ok(UiAction::ToggleFocusedFolderSelection)
        }
        runtime_contract::UiAction::MoveFolderFocus { delta } => {
            Ok(UiAction::MoveFolderFocus { delta })
        }
        runtime_contract::UiAction::StartNewFolder => Ok(UiAction::StartNewFolder),
        runtime_contract::UiAction::StartNewFolderAtFolderRow { index } => {
            Ok(UiAction::StartNewFolderAtFolderRow { index })
        }
        runtime_contract::UiAction::StartNewFolderAtRoot => Ok(UiAction::StartNewFolderAtRoot),
        runtime_contract::UiAction::FocusFolderCreateInput => Ok(UiAction::FocusFolderCreateInput),
        runtime_contract::UiAction::SetFolderCreateInput { value } => {
            Ok(UiAction::SetFolderCreateInput { value })
        }
        runtime_contract::UiAction::ConfirmFolderCreate => Ok(UiAction::ConfirmFolderCreate),
        runtime_contract::UiAction::CancelFolderCreate => Ok(UiAction::CancelFolderCreate),
        runtime_contract::UiAction::StartFolderRename => Ok(UiAction::StartFolderRename),
        runtime_contract::UiAction::DeleteFocusedFolder => Ok(UiAction::DeleteFocusedFolder),
        runtime_contract::UiAction::RestoreRetainedFolderDeletes => {
            Ok(UiAction::RestoreRetainedFolderDeletes)
        }
        runtime_contract::UiAction::PurgeRetainedFolderDeletes => {
            Ok(UiAction::PurgeRetainedFolderDeletes)
        }
        runtime_contract::UiAction::ClearFolderDeleteRecoveryLog => {
            Ok(UiAction::ClearFolderDeleteRecoveryLog)
        }
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::SelectColumn { index } => Ok(runtime_contract::UiAction::SelectColumn { index }),
        UiAction::MoveColumn { delta } => Ok(runtime_contract::UiAction::MoveColumn { delta }),
        UiAction::ToggleTransport => Ok(runtime_contract::UiAction::ToggleTransport),
        UiAction::PlayCompareAnchor => Ok(runtime_contract::UiAction::PlayCompareAnchor),
        UiAction::PlayFromStart => Ok(runtime_contract::UiAction::PlayFromStart),
        UiAction::PlayFromCurrentPlayhead => {
            Ok(runtime_contract::UiAction::PlayFromCurrentPlayhead)
        }
        UiAction::PlayFromWaveformCursor => Ok(runtime_contract::UiAction::PlayFromWaveformCursor),
        UiAction::PlayWaveformAtPrecise { position_nanos } => {
            Ok(runtime_contract::UiAction::PlayWaveformAtPrecise { position_nanos })
        }
        UiAction::HandleEscape => Ok(runtime_contract::UiAction::HandleEscape),
        UiAction::FocusBrowserPanel => Ok(runtime_contract::UiAction::FocusBrowserPanel),
        UiAction::FocusSourcesPanel => Ok(runtime_contract::UiAction::FocusSourcesPanel),
        UiAction::FocusWaveformPanel => Ok(runtime_contract::UiAction::FocusWaveformPanel),
        UiAction::FocusFolderPanel => Ok(runtime_contract::UiAction::FocusFolderPanel),
        UiAction::FocusLoadedSampleInBrowser => {
            Ok(runtime_contract::UiAction::FocusLoadedContentInList)
        }
        UiAction::FocusBrowserSearch => Ok(runtime_contract::UiAction::FocusBrowserSearch),
        UiAction::BlurBrowserSearch => Ok(runtime_contract::UiAction::BlurBrowserSearch),
        UiAction::OpenAddSourceDialog => Ok(runtime_contract::UiAction::OpenAddSourceDialog),
        UiAction::OpenOptionsMenu => Ok(runtime_contract::UiAction::OpenOptionsMenu),
        UiAction::CloseOptionsPanel => Ok(runtime_contract::UiAction::CloseOptionsPanel),
        UiAction::PickTrashFolder => Ok(runtime_contract::UiAction::PickTrashFolder),
        UiAction::OpenTrashFolder => Ok(runtime_contract::UiAction::OpenTrashFolder),
        UiAction::EditDefaultIdentifier => Ok(runtime_contract::UiAction::EditDefaultIdentifier),
        UiAction::ShowOptionsOverview => Ok(runtime_contract::UiAction::ShowOptionsOverview),
        UiAction::OpenAudioOutputHostPicker => {
            Ok(runtime_contract::UiAction::OpenPrimaryGroupPicker)
        }
        UiAction::OpenAudioOutputDevicePicker => {
            Ok(runtime_contract::UiAction::OpenPrimaryItemPicker)
        }
        UiAction::OpenAudioOutputSampleRatePicker => {
            Ok(runtime_contract::UiAction::OpenPrimaryNumberPicker)
        }
        UiAction::OpenAudioInputHostPicker => {
            Ok(runtime_contract::UiAction::OpenSecondaryGroupPicker)
        }
        UiAction::OpenAudioInputDevicePicker => {
            Ok(runtime_contract::UiAction::OpenSecondaryItemPicker)
        }
        UiAction::OpenAudioInputSampleRatePicker => {
            Ok(runtime_contract::UiAction::OpenSecondaryNumberPicker)
        }
        UiAction::SetAudioOutputHost { host_id } => {
            Ok(runtime_contract::UiAction::SetPrimaryGroup { group_id: host_id })
        }
        UiAction::SetAudioOutputDevice { device_name } => {
            Ok(runtime_contract::UiAction::SetPrimaryItem {
                item_name: device_name,
            })
        }
        UiAction::SetAudioOutputSampleRate { sample_rate } => {
            Ok(runtime_contract::UiAction::SetPrimaryNumber { value: sample_rate })
        }
        UiAction::SetAudioInputHost { host_id } => {
            Ok(runtime_contract::UiAction::SetSecondaryGroup { group_id: host_id })
        }
        UiAction::SetAudioInputDevice { device_name } => {
            Ok(runtime_contract::UiAction::SetSecondaryItem {
                item_name: device_name,
            })
        }
        UiAction::SetAudioInputSampleRate { sample_rate } => {
            Ok(runtime_contract::UiAction::SetSecondaryNumber { value: sample_rate })
        }
        UiAction::FocusFolderSearch => Ok(runtime_contract::UiAction::FocusFolderSearch),
        UiAction::SetFolderSearch { query } => {
            Ok(runtime_contract::UiAction::SetFolderSearch { query })
        }
        UiAction::ToggleShowAllFolders => Ok(runtime_contract::UiAction::ToggleShowAllFolders),
        UiAction::ToggleFolderFlattenedView => {
            Ok(runtime_contract::UiAction::ToggleFolderFlattenedView)
        }
        UiAction::FocusSourceRow { index } => {
            Ok(runtime_contract::UiAction::FocusSourceRow { index })
        }
        UiAction::SelectSourceRow { index } => {
            Ok(runtime_contract::UiAction::SelectSourceRow { index })
        }
        UiAction::MoveSourceFocus { delta } => {
            Ok(runtime_contract::UiAction::MoveSourceFocus { delta })
        }
        UiAction::ReloadFocusedSourceRow => Ok(runtime_contract::UiAction::ReloadFocusedSourceRow),
        UiAction::HardSyncFocusedSourceRow => {
            Ok(runtime_contract::UiAction::HardSyncFocusedSourceRow)
        }
        UiAction::OpenFocusedSourceFolder => {
            Ok(runtime_contract::UiAction::OpenFocusedSourceFolder)
        }
        UiAction::RemoveFocusedSourceRow => Ok(runtime_contract::UiAction::RemoveFocusedSourceRow),
        UiAction::ReloadSourceRow { index } => {
            Ok(runtime_contract::UiAction::ReloadSourceRow { index })
        }
        UiAction::HardSyncSourceRow { index } => {
            Ok(runtime_contract::UiAction::HardSyncSourceRow { index })
        }
        UiAction::OpenSourceFolderRow { index } => {
            Ok(runtime_contract::UiAction::OpenSourceFolderRow { index })
        }
        UiAction::RemoveSourceRow { index } => {
            Ok(runtime_contract::UiAction::RemoveSourceRow { index })
        }
        UiAction::FocusFolderRow { index } => {
            Ok(runtime_contract::UiAction::FocusFolderRow { index })
        }
        UiAction::ActivateFolderRow { index } => {
            Ok(runtime_contract::UiAction::ActivateFolderRow { index })
        }
        UiAction::ToggleFolderRowExpanded { index } => {
            Ok(runtime_contract::UiAction::ToggleFolderRowExpanded { index })
        }
        UiAction::ExpandFocusedFolder => Ok(runtime_contract::UiAction::ExpandFocusedFolder),
        UiAction::CollapseFocusedFolder => Ok(runtime_contract::UiAction::CollapseFocusedFolder),
        UiAction::ToggleFocusedFolderSelection => {
            Ok(runtime_contract::UiAction::ToggleFocusedFolderSelection)
        }
        UiAction::MoveFolderFocus { delta } => {
            Ok(runtime_contract::UiAction::MoveFolderFocus { delta })
        }
        UiAction::StartNewFolder => Ok(runtime_contract::UiAction::StartNewFolder),
        UiAction::StartNewFolderAtFolderRow { index } => {
            Ok(runtime_contract::UiAction::StartNewFolderAtFolderRow { index })
        }
        UiAction::StartNewFolderAtRoot => Ok(runtime_contract::UiAction::StartNewFolderAtRoot),
        UiAction::FocusFolderCreateInput => Ok(runtime_contract::UiAction::FocusFolderCreateInput),
        UiAction::SetFolderCreateInput { value } => {
            Ok(runtime_contract::UiAction::SetFolderCreateInput { value })
        }
        UiAction::ConfirmFolderCreate => Ok(runtime_contract::UiAction::ConfirmFolderCreate),
        UiAction::CancelFolderCreate => Ok(runtime_contract::UiAction::CancelFolderCreate),
        UiAction::StartFolderRename => Ok(runtime_contract::UiAction::StartFolderRename),
        UiAction::DeleteFocusedFolder => Ok(runtime_contract::UiAction::DeleteFocusedFolder),
        UiAction::RestoreRetainedFolderDeletes => {
            Ok(runtime_contract::UiAction::RestoreRetainedFolderDeletes)
        }
        UiAction::PurgeRetainedFolderDeletes => {
            Ok(runtime_contract::UiAction::PurgeRetainedFolderDeletes)
        }
        UiAction::ClearFolderDeleteRecoveryLog => {
            Ok(runtime_contract::UiAction::ClearFolderDeleteRecoveryLog)
        }
        other => Err(other),
    }
}
