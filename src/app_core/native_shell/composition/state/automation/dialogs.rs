//! Dialog and overlay automation snapshot builders.

use super::helpers::{
    action_slug, bounds, metadata, node_id, simple_node, slug, update_status_text,
};
use super::*;
use crate::compat_app_contract::AutomationRole;

/// Build semantic automation for the update panel embedded in the top bar.
pub(super) fn update_panel_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let surface = resolve_top_bar_surface_layout(
        layout.top_bar,
        style.sizing,
        &top_bar_surface_content(model),
    );
    let mut children = Vec::new();
    for button in surface.update_buttons {
        children.push(simple_node(
            format!("shell.top_bar.update.{}", button.spec.node_slug),
            AutomationRole::Button,
            Some(String::from(button.spec.label)),
            button.rect,
            None,
            button.spec.enabled,
            false,
            vec![action_slug(&button.spec.action)],
        ));
    }
    AutomationNodeSnapshot {
        id: node_id("shell.top_bar.update_panel"),
        role: AutomationRole::Group,
        label: Some(String::from("Updates")),
        bounds: bounds(surface.action_cluster),
        value: Some(model.update.status_label.clone()),
        enabled: true,
        selected: !children.is_empty(),
        available_actions: children
            .iter()
            .flat_map(|child| child.available_actions.clone())
            .collect(),
        metadata: metadata(&[
            ("status", update_status_text(model.update.status)),
            ("status_label", model.update.status_label.as_str()),
            ("action_hint", model.update.action_hint_label.as_str()),
            ("release_notes", model.update.release_notes_label.as_str()),
            (
                "available_version_label",
                model
                    .update
                    .available_version_label
                    .as_deref()
                    .unwrap_or(""),
            ),
            (
                "available_url",
                model.update.available_url.as_deref().unwrap_or(""),
            ),
            (
                "last_error",
                model.update.last_error.as_deref().unwrap_or(""),
            ),
        ]),
        children,
    }
}

/// Build semantic automation for the options overlay when it is visible.
pub(super) fn options_panel_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    let panel = options_panel_layout(layout, style, model)?;
    Some(AutomationNodeSnapshot {
        id: node_id("overlay.options_panel"),
        role: AutomationRole::Dialog,
        label: Some(String::from("Options")),
        bounds: bounds(panel.panel_rect),
        value: None,
        enabled: true,
        selected: true,
        available_actions: vec![String::from("close_options_panel")],
        metadata: std::collections::BTreeMap::new(),
        children: panel
            .buttons
            .into_iter()
            .map(|button| {
                simple_node(
                    format!("overlay.options_panel.{}", slug(&button.text)),
                    AutomationRole::Button,
                    Some(button.text),
                    button.rect,
                    None,
                    true,
                    button.active,
                    vec![action_slug(&button.action)],
                )
            })
            .collect(),
    })
}

/// Build semantic automation for the confirm prompt overlay when it is visible.
pub(super) fn prompt_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    if !model.confirm_prompt.visible {
        return None;
    }
    let (confirm_button, cancel_button) = prompt_buttons(layout, style);
    let dialog = compute_prompt_overlay_visual_layout(
        layout.root.rect,
        layout.content,
        style.sizing,
        model.confirm_prompt.input_value.is_some(),
        model.confirm_prompt.target_label.is_some(),
    )
    .sections
    .dialog;
    let mut children = vec![
        simple_node(
            "overlay.prompt.confirm",
            AutomationRole::Button,
            Some(model.confirm_prompt.confirm_label.clone()),
            confirm_button,
            None,
            model.confirm_prompt.input_error.is_none(),
            false,
            vec![String::from("confirm_prompt")],
        ),
        simple_node(
            "overlay.prompt.cancel",
            AutomationRole::Button,
            Some(model.confirm_prompt.cancel_label.clone()),
            cancel_button,
            None,
            true,
            false,
            vec![String::from("cancel_prompt")],
        ),
    ];
    if let Some(input_rect) = prompt_input_rect(layout, style, model) {
        children.push(simple_node(
            "overlay.prompt.input",
            AutomationRole::SearchField,
            Some(String::from("Prompt input")),
            input_rect,
            model.confirm_prompt.input_value.clone(),
            true,
            false,
            vec![String::from("set_prompt_input")],
        ));
    }
    Some(AutomationNodeSnapshot {
        id: node_id("overlay.prompt"),
        role: AutomationRole::Dialog,
        label: Some(model.confirm_prompt.title.clone()),
        bounds: bounds(dialog),
        value: Some(model.confirm_prompt.message.clone()),
        enabled: true,
        selected: true,
        available_actions: Vec::new(),
        metadata: metadata(&[
            ("kind", &format!("{:?}", model.confirm_prompt.kind)),
            (
                "input_error",
                model.confirm_prompt.input_error.as_deref().unwrap_or(""),
            ),
        ]),
        children,
    })
}

/// Build semantic automation for the modal progress overlay when it is visible.
pub(super) fn progress_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    if !model.progress_overlay.visible || !model.progress_overlay.modal {
        return None;
    }
    Some(AutomationNodeSnapshot {
        id: node_id("overlay.progress"),
        role: AutomationRole::Dialog,
        label: Some(model.progress_overlay.title.clone()),
        bounds: bounds(
            compute_progress_overlay_visual_layout(
                layout.root.rect,
                layout.content,
                style.sizing,
                true,
                0.0,
            )
            .sections
            .dialog,
        ),
        value: model.progress_overlay.detail.clone(),
        enabled: true,
        selected: true,
        available_actions: Vec::new(),
        metadata: metadata(&[
            ("completed", &model.progress_overlay.completed.to_string()),
            ("total", &model.progress_overlay.total.to_string()),
        ]),
        children: if model.progress_overlay.cancelable {
            vec![simple_node(
                "overlay.progress.cancel",
                AutomationRole::Button,
                Some(String::from("Cancel")),
                progress_cancel_button(layout, style, true),
                None,
                !model.progress_overlay.cancel_requested,
                false,
                vec![String::from("cancel_progress")],
            )]
        } else {
            Vec::new()
        },
    })
}
