use super::*;

impl From<runtime_contract::ConfirmPromptKind> for ConfirmPromptKind {
    fn from(value: runtime_contract::ConfirmPromptKind) -> Self {
        match value {
            runtime_contract::ConfirmPromptKind::DestructiveOperation => Self::DestructiveEdit,
            runtime_contract::ConfirmPromptKind::RenameContent => Self::BrowserRename,
            runtime_contract::ConfirmPromptKind::RenameNavigationItem => Self::FolderRename,
            runtime_contract::ConfirmPromptKind::CreateNavigationItem => Self::FolderCreate,
            runtime_contract::ConfirmPromptKind::RestoreRetainedItems => {
                Self::RestoreRetainedFolderDeletes
            }
            runtime_contract::ConfirmPromptKind::PurgeRetainedItems => {
                Self::PurgeRetainedFolderDeletes
            }
            runtime_contract::ConfirmPromptKind::EditConfiguration => {
                Self::OptionsDefaultIdentifier
            }
        }
    }
}

impl From<ConfirmPromptKind> for runtime_contract::ConfirmPromptKind {
    fn from(value: ConfirmPromptKind) -> Self {
        match value {
            ConfirmPromptKind::DestructiveEdit => Self::DestructiveOperation,
            ConfirmPromptKind::BrowserRename => Self::RenameContent,
            ConfirmPromptKind::FolderRename => Self::RenameNavigationItem,
            ConfirmPromptKind::FolderCreate => Self::CreateNavigationItem,
            ConfirmPromptKind::RestoreRetainedFolderDeletes => Self::RestoreRetainedItems,
            ConfirmPromptKind::PurgeRetainedFolderDeletes => Self::PurgeRetainedItems,
            ConfirmPromptKind::OptionsDefaultIdentifier => Self::EditConfiguration,
        }
    }
}

pub(super) fn confirm_prompt_from_compat(
    value: runtime_contract::ConfirmPromptModel,
) -> ConfirmPromptModel {
    ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

pub(super) fn confirm_prompt_to_compat(
    value: ConfirmPromptModel,
) -> runtime_contract::ConfirmPromptModel {
    runtime_contract::ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

impl From<&WaveformPanelModel> for runtime_contract::WaveformPanelModel {
    fn from(value: &WaveformPanelModel) -> Self {
        value.clone()
    }
}

impl From<&WaveformChromeModel> for runtime_contract::WaveformChromeModel {
    fn from(value: &WaveformChromeModel) -> Self {
        value.clone()
    }
}
