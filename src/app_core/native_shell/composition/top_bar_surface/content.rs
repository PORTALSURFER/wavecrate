use crate::{
    app::{AppModel, UiAction, UpdateStatusModel},
    gui::types::Rect,
};

/// User-facing content projected into the generic top-bar surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarSurfaceContent {
    /// Product title projected into the compact chrome band.
    pub title: String,
    /// Formatted master-volume value.
    pub volume_value: String,
    /// Short volume label paired with the meter.
    pub volume_label: String,
    /// Compact audio-engine chip label shown on the options button.
    pub options_label: String,
    /// Bounded update actions projected into the action cluster.
    pub update_actions: Vec<TopBarUpdateActionSpec>,
}

/// One projected update action hosted inside the generic top-bar surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarUpdateActionSpec {
    /// Stable automation slug used for semantic node ids.
    pub node_slug: &'static str,
    /// User-facing action label.
    pub label: &'static str,
    /// Native action emitted when the button activates.
    pub action: UiAction,
    /// Whether the action is currently interactive.
    pub enabled: bool,
}

/// Resolved button geometry for one projected update action.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarUpdateButtonLayout {
    /// Action metadata associated with the rendered button.
    pub spec: TopBarUpdateActionSpec,
    /// Resolved button bounds.
    pub rect: Rect,
}

/// Resolved layout rectangles for the generic top-bar surface.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TopBarSurfaceLayout {
    /// Left host cluster that contains the volume affordance and title copy.
    pub title_cluster: Rect,
    /// Right host cluster that contains update actions and the options button.
    pub action_cluster: Rect,
    /// Title text widget bounds.
    pub title_text_rect: Rect,
    /// Volume meter canvas bounds.
    pub volume_meter_rect: Rect,
    /// Volume value text bounds.
    pub volume_value_rect: Rect,
    /// Volume label text bounds.
    pub volume_label_rect: Rect,
    /// Options button bounds, when enough room remains.
    pub options_button_rect: Option<Rect>,
    /// Visible update action buttons resolved inside the action cluster.
    pub update_buttons: Vec<TopBarUpdateButtonLayout>,
}

/// Build user-facing top-bar surface content from the projected app model.
pub(crate) fn top_bar_surface_content(model: &AppModel) -> TopBarSurfaceContent {
    TopBarSurfaceContent {
        title: model.title.clone(),
        volume_value: format!("{:.2}", model.volume.clamp(0.0, 1.0)),
        volume_label: String::from("Vol"),
        options_label: model.paired_device_panel().status_label().to_string(),
        update_actions: top_bar_update_action_specs(model),
    }
}

/// Build the update-action descriptors projected into the top-bar chrome.
pub(crate) fn top_bar_update_action_specs(model: &AppModel) -> Vec<TopBarUpdateActionSpec> {
    match model.update.status {
        UpdateStatusModel::Idle => vec![TopBarUpdateActionSpec {
            node_slug: "check",
            label: "Check",
            action: UiAction::CheckForUpdates,
            enabled: true,
        }],
        UpdateStatusModel::Checking => Vec::new(),
        UpdateStatusModel::Available => {
            let mut buttons = Vec::new();
            if model.update.available_url.is_some() {
                buttons.push(TopBarUpdateActionSpec {
                    node_slug: "open",
                    label: "Open",
                    action: UiAction::OpenUpdateLink,
                    enabled: true,
                });
                buttons.push(TopBarUpdateActionSpec {
                    node_slug: "install",
                    label: "Install",
                    action: UiAction::InstallUpdate,
                    enabled: true,
                });
            }
            buttons.push(TopBarUpdateActionSpec {
                node_slug: "dismiss",
                label: "Dismiss",
                action: UiAction::DismissUpdate,
                enabled: true,
            });
            buttons
        }
        UpdateStatusModel::Error => vec![TopBarUpdateActionSpec {
            node_slug: "check",
            label: "Retry",
            action: UiAction::CheckForUpdates,
            enabled: true,
        }],
    }
}
