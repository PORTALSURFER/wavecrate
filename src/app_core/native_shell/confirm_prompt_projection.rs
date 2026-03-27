//! Confirm-prompt projection helpers.

use super::*;

/// Project active confirm prompt metadata for modal rendering.
pub(crate) fn project_confirm_prompt_model(ui: &UiState) -> ConfirmPromptModel {
    if let Some(SampleBrowserActionPrompt::Rename { target, name }) =
        ui.browser.pending_action.clone()
    {
        let input_value = Some(name);
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::BrowserRename),
            title: String::from("Rename sample"),
            message: String::from("Apply rename for focused sample?"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target.display().to_string()),
            input_value,
            input_placeholder: Some(String::from("Sample name")),
            input_error: None,
        };
    }
    if let Some(FolderActionPrompt::RestoreRetainedDeletes { entry_count }) =
        ui.sources.folders.pending_action.clone()
    {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::RestoreRetainedFolderDeletes),
            title: String::from("Restore retained deletes"),
            message: format!("Restore {entry_count} retained folder delete(s) from Recovery?"),
            confirm_label: String::from("Restore"),
            cancel_label: String::from("Cancel"),
            target_label: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
        };
    }
    if let Some(FolderActionPrompt::PurgeRetainedDeletes { entry_count }) =
        ui.sources.folders.pending_action.clone()
    {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::PurgeRetainedFolderDeletes),
            title: String::from("Purge retained deletes"),
            message: format!(
                "Permanently purge {entry_count} retained folder delete(s) from Recovery?"
            ),
            confirm_label: String::from("Purge"),
            cancel_label: String::from("Cancel"),
            target_label: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
        };
    }
    if let Some(prompt) = ui.waveform.pending_destructive.clone() {
        return ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::DestructiveEdit),
            title: prompt.title,
            message: prompt.message,
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: None,
            input_value: None,
            input_placeholder: None,
            input_error: None,
        };
    }
    ConfirmPromptModel::default()
}
