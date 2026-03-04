//! Update-panel projection helpers.

use super::*;

/// Project update panel state into the native shell model.
pub(crate) fn project_update_model(ui: &UiState) -> UpdatePanelModel {
    let update_status = UpdateStatus::from(ui.update.status.clone());
    let status = match update_status {
        UpdateStatus::Idle => UpdateStatusModel::Idle,
        UpdateStatus::Checking => UpdateStatusModel::Checking,
        UpdateStatus::UpdateAvailable => UpdateStatusModel::Available,
        UpdateStatus::Error => UpdateStatusModel::Error,
    };
    let status_label = match update_status {
        UpdateStatus::Idle => String::from("Updates: idle"),
        UpdateStatus::Checking => String::from("Checking updates..."),
        UpdateStatus::UpdateAvailable => ui
            .update
            .available_tag
            .as_deref()
            .map(|tag| format!("Update available: {tag} (manual install required)"))
            .unwrap_or_else(|| String::from("Update available (manual install required)")),
        UpdateStatus::Error => ui
            .update
            .last_error
            .as_deref()
            .map(|err| format!("Update check failed: {err}"))
            .unwrap_or_else(|| String::from("Update check failed")),
    };
    let action_hint_label = match update_status {
        UpdateStatus::Idle => String::from("Action: check"),
        UpdateStatus::Checking => String::from("Action: waiting"),
        UpdateStatus::UpdateAvailable => {
            if ui.update.available_url.is_some() {
                String::from("Actions: open | install(manual) | dismiss")
            } else {
                String::from("Action: dismiss")
            }
        }
        UpdateStatus::Error => String::from("Action: retry"),
    };
    let release_notes_label = match update_status {
        UpdateStatus::UpdateAvailable => {
            let tag = ui.update.available_tag.as_deref().unwrap_or("latest");
            if let Some(published_at) = ui.update.available_published_at.as_deref() {
                format!("Release: {tag} ({published_at}) | Signed manual install required")
            } else {
                format!("Release: {tag} | Signed manual install required")
            }
        }
        _ => String::new(),
    };
    UpdatePanelModel {
        status,
        status_label,
        action_hint_label,
        release_notes_label,
        available_tag: ui.update.available_tag.clone(),
        available_url: ui.update.available_url.clone(),
        last_error: ui.update.last_error.clone(),
    }
}
