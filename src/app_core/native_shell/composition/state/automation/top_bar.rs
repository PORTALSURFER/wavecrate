//! Top-bar automation snapshot builders.

use super::helpers::{metadata, simple_node};
use super::*;
use crate::compat_app_contract::AutomationRole;

/// Build semantic automation for the top bar and embedded update panel.
pub(super) fn build_top_bar_automation(
    _shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
) -> AutomationNodeSnapshot {
    let style = style_for_layout(layout);
    let surface = resolve_top_bar_surface_layout(
        layout.top_bar,
        style.sizing,
        &top_bar_surface_content(model),
    );
    let mut children = Vec::new();
    if surface.volume_meter_rect.width() > 0.0 && surface.volume_meter_rect.height() > 0.0 {
        children.push(simple_node(
            "shell.top_bar.volume_slider",
            AutomationRole::Slider,
            Some(String::from("Volume")),
            surface.volume_meter_rect,
            Some(format!("{:.3}", model.volume.clamp(0.0, 1.0))),
            true,
            false,
            vec![
                String::from("set_volume"),
                String::from("commit_volume_setting"),
            ],
        ));
    }
    if let Some(button_rect) = surface.options_button_rect {
        children.push(simple_node(
            "shell.top_bar.options_button",
            AutomationRole::Button,
            Some(String::from("Audio Engine")),
            button_rect,
            Some(model.paired_device_panel().status_label().to_string()),
            true,
            model.options_panel.visible,
            vec![String::from(if model.options_panel.visible {
                "close_options_panel"
            } else {
                "open_options_menu"
            })],
        ));
    }
    children.push(super::dialogs::update_panel_automation(
        layout, model, &style,
    ));
    AutomationNodeSnapshot {
        id: super::helpers::node_id("shell.top_bar"),
        role: AutomationRole::Panel,
        label: Some(String::from("Top bar")),
        bounds: super::helpers::bounds(layout.top_bar),
        value: Some(model.title.clone()),
        enabled: true,
        selected: false,
        available_actions: Vec::new(),
        metadata: metadata(&[
            ("title", model.title.as_str()),
            ("backend", model.backend_label.as_str()),
        ]),
        children,
    }
}
