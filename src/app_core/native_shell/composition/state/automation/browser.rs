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

use map::map_canvas_automation;
use pill_editor::build_browser_pill_editor_automation;
use table::build_browser_table_automation;

/// Build semantic automation for the browser panel and its active content.
pub(super) fn build_browser_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let toolbar = browser_toolbar_layout(layout, style, model);
    let buttons = browser_action_buttons(layout, style, model, &toolbar);
    let tabs = resolve_browser_tabs_surface_layout(
        layout.browser_tabs,
        style.sizing,
        &browser_tabs_surface_content(model),
    );
    let mut children = vec![
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
    ];
    if toolbar.search_field.width() > 1.0 {
        children.push(simple_node(
            "browser.search_field",
            AutomationRole::SearchField,
            Some(String::from("Browser search")),
            toolbar.search_field,
            Some(model.browser.search_query.clone()),
            true,
            false,
            vec![
                String::from("focus_browser_search"),
                String::from("set_browser_search"),
            ],
        ));
    }
    for (index, chip) in toolbar.rating_filter_chips.iter().copied().enumerate() {
        if chip.width() <= 1.0 {
            continue;
        }
        let level = super::super::BROWSER_RATING_FILTER_LEVELS[index];
        children.push(simple_node(
            format!("browser.rating_filter.{level}"),
            AutomationRole::Button,
            Some(format!("Rating filter {level}")),
            chip,
            None,
            true,
            model.browser.active_rating_filters[index],
            vec![String::from("toggle_browser_rating_filter")],
        ));
    }
    for (index, chip) in toolbar
        .playback_age_filter_chips
        .iter()
        .copied()
        .enumerate()
    {
        if chip.width() <= 1.0 {
            continue;
        }
        let bucket = super::super::BROWSER_PLAYBACK_AGE_FILTER_CHIPS[index];
        let (slug, label) = match bucket {
            crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip::NeverPlayed => {
                ("never", "Never played")
            }
            crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip::OlderThanMonth => {
                ("month", "Older than month")
            }
            crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip::OlderThanWeek => {
                ("week", "Older than week")
            }
        };
        children.push(simple_node(
            format!("browser.playback_age_filter.{slug}"),
            AutomationRole::Button,
            Some(String::from(label)),
            chip,
            None,
            true,
            model.browser.active_recency_filters[index],
            vec![String::from("toggle_browser_playback_age_filter")],
        ));
    }
    if toolbar.marked_filter_chip.width() > 1.0 {
        children.push(simple_node(
            "browser.marked_filter",
            AutomationRole::Button,
            Some(String::from("Marked filter")),
            toolbar.marked_filter_chip,
            None,
            true,
            model.browser.marked_filter_active,
            vec![String::from("toggle_browser_marked_filter")],
        ));
    }
    if toolbar.derived_label_filter_chip.width() > 1.0 {
        children.push(simple_node(
            "browser.derived_label_filter",
            AutomationRole::Button,
            Some(String::from("Derived-label filter")),
            toolbar.derived_label_filter_chip,
            None,
            true,
            model.browser.derived_label_filter_active,
            vec![String::from("toggle_browser_derived_label_filter")],
        ));
    }
    for button in buttons {
        children.push(simple_node(
            format!("browser.action.{}", slug(button.label)),
            AutomationRole::Button,
            Some(String::from(button.label)),
            button.rect,
            None,
            button.enabled,
            button.active,
            vec![action_slug(&button.action)],
        ));
    }
    if model.map.active {
        children.push(map_canvas_automation(layout, model, style));
    } else {
        children.push(build_browser_table_automation(shell, layout, model, style));
        if let Some(pill_editor) = build_browser_pill_editor_automation(layout, model, style) {
            children.push(pill_editor);
        }
    }
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
