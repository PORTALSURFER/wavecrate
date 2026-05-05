//! Deterministic automation snapshot builders for the native shell.

use super::*;
use crate::compat_app_contract::{AutomationNodeSnapshot, AutomationRole, GuiAutomationSnapshot};
use std::collections::BTreeMap;

#[path = "automation/browser.rs"]
mod browser;
#[path = "automation/dialogs.rs"]
mod dialogs;
#[path = "automation/helpers.rs"]
mod helpers;
#[path = "automation/sidebar.rs"]
mod sidebar;
#[path = "automation/top_bar.rs"]
mod top_bar;
#[path = "automation/waveform.rs"]
mod waveform;

use self::{
    browser::build_browser_automation,
    dialogs::{options_panel_automation, progress_automation, prompt_automation},
    helpers::{bounds, node_id},
    sidebar::build_sidebar_automation,
    top_bar::build_top_bar_automation,
    waveform::build_waveform_automation,
};

impl NativeShellState {
    /// Build a deterministic semantic automation snapshot for the current shell state.
    pub(crate) fn automation_snapshot(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> GuiAutomationSnapshot {
        let viewport_width =
            u32::try_from(layout.root.rect.width().round().max(1.0) as i64).unwrap_or(1);
        let viewport_height =
            u32::try_from(layout.root.rect.height().round().max(1.0) as i64).unwrap_or(1);
        GuiAutomationSnapshot {
            schema_version: 1,
            viewport_width,
            viewport_height,
            root: AutomationNodeSnapshot {
                id: node_id("shell.root"),
                role: AutomationRole::Root,
                label: Some(if model.title.trim().is_empty() {
                    String::from("Radiant shell")
                } else {
                    format!("{} shell", model.title)
                }),
                bounds: bounds(layout.root.rect),
                value: None,
                enabled: true,
                selected: false,
                available_actions: Vec::new(),
                metadata: BTreeMap::new(),
                children: self.automation_children(layout, model),
            },
        }
    }

    fn automation_children(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<AutomationNodeSnapshot> {
        let style = style_for_layout(layout);
        vec![
            build_top_bar_automation(self, layout, model),
            build_sidebar_automation(self, layout, model, &style),
            build_waveform_automation(self, layout, model, &style),
            build_browser_automation(self, layout, model, &style),
            self.status_bar_automation(layout, model),
        ]
        .into_iter()
        .chain(options_panel_automation(layout, model, &style))
        .chain(prompt_automation(layout, model, &style))
        .chain(progress_automation(layout, model, &style))
        .collect()
    }

    fn status_bar_automation(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> AutomationNodeSnapshot {
        AutomationNodeSnapshot {
            id: node_id("shell.status_bar"),
            role: AutomationRole::Readout,
            label: Some(String::from("Status bar")),
            bounds: bounds(layout.status_bar),
            value: Some(model.status.center.clone()),
            enabled: true,
            selected: false,
            available_actions: Vec::new(),
            metadata: helpers::metadata(&[
                ("left", model.status.left.as_str()),
                ("center", model.status.center.as_str()),
                ("right", model.status.right.as_str()),
            ]),
            children: Vec::new(),
        }
    }
}
