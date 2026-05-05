//! Confirm-prompt projection helpers.

use super::*;
use crate::app_core::state::OptionsPanelPrompt;

/// Project active confirm prompt metadata for modal rendering.
pub(crate) fn project_confirm_prompt_model(ui: &UiState) -> ConfirmPromptModel {
    if let Some(prompt) = ui.browser.pending_action.clone() {
        return project_browser_prompt(prompt);
    }
    if let Some(prompt) = ui.options_panel.pending_prompt.clone() {
        return project_options_panel_prompt(prompt);
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

fn project_options_panel_prompt(prompt: OptionsPanelPrompt) -> ConfirmPromptModel {
    match prompt {
        OptionsPanelPrompt::DefaultIdentifier { value } => ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::OptionsDefaultIdentifier),
            title: String::from("Default identifier"),
            message: String::from("Set the first tag used by Auto Rename."),
            confirm_label: String::from("Save"),
            cancel_label: String::from("Cancel"),
            target_label: None,
            input_value: Some(value),
            input_placeholder: Some(String::from("portal")),
            input_error: None,
        },
    }
}

fn project_browser_prompt(prompt: SampleBrowserActionPrompt) -> ConfirmPromptModel {
    match prompt {
        SampleBrowserActionPrompt::Rename {
            target,
            name,
            input_error,
        } => ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::BrowserRename),
            title: String::from("Rename sample"),
            message: String::from("Apply rename for focused sample?"),
            confirm_label: String::from("Apply"),
            cancel_label: String::from("Cancel"),
            target_label: Some(target.display().to_string()),
            input_value: Some(name),
            input_placeholder: Some(String::from("Sample name")),
            input_error,
        },
        SampleBrowserActionPrompt::MoveToFolderConflict {
            target_folder,
            name,
            input_error,
            ..
        } => ConfirmPromptModel {
            visible: true,
            kind: Some(ConfirmPromptKind::BrowserRename),
            title: String::from("Name conflict"),
            message: String::from(
                "That folder already contains a file with this name. Choose a new name to finish the drop.",
            ),
            confirm_label: String::from("Move"),
            cancel_label: String::from("Cancel"),
            target_label: Some(folder_drop_target_label(&target_folder)),
            input_value: Some(name),
            input_placeholder: Some(String::from("Sample name")),
            input_error,
        },
    }
}

fn folder_drop_target_label(target_folder: &std::path::Path) -> String {
    if target_folder.as_os_str().is_empty() {
        String::from("Source root")
    } else {
        format!("Folder: {}", target_folder.display())
    }
}
