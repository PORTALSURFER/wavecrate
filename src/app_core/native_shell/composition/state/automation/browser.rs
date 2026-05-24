//! Browser-panel automation snapshot builders.

use super::action_slugs::action_slug;
use super::helpers::{bool_text, bounds, circle_rect, metadata, node_id, simple_node, slug};
use super::*;
use crate::app_core::native_shell::runtime_contract::AutomationRole;

#[path = "browser/map.rs"]
mod map;
#[path = "browser/pill_editor.rs"]
mod pill_editor;
#[path = "browser/table.rs"]
mod table;
#[path = "browser/toolbar.rs"]
mod toolbar;

use map::map_canvas_automation;
use pill_editor::build_browser_pill_editor_automation;
use table::build_browser_table_automation;
use toolbar::{browser_action_nodes, toolbar_control_nodes};

/// Build semantic automation for the browser panel and its active content.
pub(super) fn build_browser_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let toolbar = browser_toolbar_layout(layout, style, model);
    let buttons = browser_action_buttons(layout, style, model, &toolbar);
    let mut children = browser_tab_nodes(layout, model, style);
    children.extend(toolbar_control_nodes(&toolbar, model));
    children.extend(browser_action_nodes(buttons));
    children.extend(browser_content_nodes(shell, layout, model, style));

    browser_panel_node(layout, model, children)
}

fn browser_tab_nodes(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<AutomationNodeSnapshot> {
    let tabs = resolve_browser_tabs_surface_layout(
        layout.browser_tabs,
        style.sizing,
        &browser_tabs_surface_content(model),
    );

    vec![
        simple_node(
            "browser.tab.items",
            AutomationRole::Tab,
            Some(model.browser_chrome.items_tab_label.clone()),
            tabs.items,
            None,
            true,
            !model.map.active,
            vec![String::from("set_browser_tab")],
        ),
        simple_node(
            "browser.tab.map",
            AutomationRole::Tab,
            Some(model.browser_chrome.map_tab_label.clone()),
            tabs.map,
            None,
            true,
            model.map.active,
            vec![String::from("set_browser_tab")],
        ),
    ]
}

fn browser_content_nodes(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Vec<AutomationNodeSnapshot> {
    let mut children = Vec::new();
    if model.map.active {
        children.push(map_canvas_automation(layout, model, style));
    } else {
        children.push(build_browser_table_automation(shell, layout, model, style));
        if let Some(pill_editor) = build_browser_pill_editor_automation(layout, model, style) {
            children.push(pill_editor);
        }
    }

    children
}

fn browser_panel_node(
    layout: &ShellLayout,
    model: &AppModel,
    children: Vec<AutomationNodeSnapshot>,
) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: node_id("browser.panel"),
        role: AutomationRole::Panel,
        label: Some(String::from("Browser panel")),
        bounds: bounds(layout.browser_panel),
        value: Some(model.browser_chrome.item_count_label.clone()),
        enabled: true,
        selected: matches!(
            model.focus_context,
            crate::app_core::native_shell::runtime_contract::FocusContextModel::ContentList
        ),
        available_actions: vec![String::from("focus_browser_panel")],
        metadata: metadata(&[
            (
                "active_tab",
                model.browser.active_tab_label.as_deref().unwrap_or(""),
            ),
            ("search_query", model.browser.search_query.as_str()),
            (
                "focused_item_label",
                model.browser.focused_item_label.as_deref().unwrap_or(""),
            ),
            (
                "selected_visible_row",
                &model
                    .browser
                    .selected_visible_row
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            ),
            (
                "random_navigation_enabled",
                if model.browser_actions.random_navigation_enabled {
                    "true"
                } else {
                    "false"
                },
            ),
        ]),
        children,
    }
}
