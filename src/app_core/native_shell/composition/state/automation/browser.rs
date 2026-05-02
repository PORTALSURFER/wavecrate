//! Browser-panel automation snapshot builders.

use super::helpers::{
    action_slug, bool_text, bounds, circle_rect, metadata, node_id, simple_node, slug,
};
use super::*;
use crate::compat_app_contract::AutomationRole;
use crate::compat_app_contract::BrowserPillState;

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
            crate::compat_app_contract::PlaybackAgeFilterChip::NeverPlayed => {
                ("never", "Never played")
            }
            crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanMonth => {
                ("month", "Older than month")
            }
            crate::compat_app_contract::PlaybackAgeFilterChip::OlderThanWeek => {
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
            crate::compat_app_contract::FocusContextModel::ContentList
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

fn build_browser_table_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let rows = shell.cached_browser_rows(layout, style, model).to_vec();
    let first_visible_row = rows
        .first()
        .map(|row| row.visible_row.to_string())
        .unwrap_or_default();
    let last_visible_row = rows
        .last()
        .map(|row| row.visible_row.to_string())
        .unwrap_or_default();
    let rendered_row_count = rows.len().to_string();
    let mut table_node = simple_node(
        "browser.table",
        AutomationRole::Table,
        Some(String::from("Browser rows")),
        layout.browser_rows,
        Some(model.browser_chrome.item_count_label.clone()),
        true,
        matches!(
            model.focus_context,
            crate::compat_app_contract::FocusContextModel::ContentList
        ),
        vec![
            String::from("focus_browser_panel"),
            String::from("set_browser_view_start"),
        ],
    );
    table_node.metadata = metadata(&[
        ("first_visible_row", &first_visible_row),
        ("last_visible_row", &last_visible_row),
        ("rendered_row_count", &rendered_row_count),
    ]);
    let mut children: Vec<_> = rows
        .into_iter()
        .map(|row| AutomationNodeSnapshot {
            id: node_id(format!("browser.row.{}", row.visible_row)),
            role: AutomationRole::Row,
            label: Some(row.label.clone()),
            bounds: bounds(row.rect),
            value: (!row.bucket_label.is_empty()).then_some(row.bucket_label.clone()),
            enabled: true,
            selected: row.selected || row.focused,
            available_actions: vec![
                String::from("focus_browser_row"),
                String::from("toggle_browser_row_selection"),
                String::from("commit_focused_browser_row"),
            ],
            metadata: metadata(&[
                ("column", &row.column.to_string()),
                ("rating_level", &row.rating_level.to_string()),
                ("focused", bool_text(row.focused)),
                ("missing", bool_text(row.missing)),
                ("locked", bool_text(row.locked)),
                ("marked", bool_text(row.marked)),
                (
                    "playback_age_bucket",
                    match row.playback_age_bucket {
                        crate::compat_app_contract::PlaybackAgeBucket::Fresh => "fresh",
                        crate::compat_app_contract::PlaybackAgeBucket::OlderThanWeek => "week",
                        crate::compat_app_contract::PlaybackAgeBucket::OlderThanMonth => "month",
                        crate::compat_app_contract::PlaybackAgeBucket::NeverPlayed => "never",
                    },
                ),
            ]),
            children: Vec::new(),
        })
        .collect();
    if let Some((scrollbar, viewport_len)) = shell.cached_browser_scrollbar(layout, model) {
        let visible_count = model.browser.visible_count.to_string();
        let viewport_len = viewport_len.to_string();
        let view_start_row = first_visible_row.clone();
        table_node.metadata.extend(metadata(&[
            ("scrollbar_visible", "true"),
            ("viewport_len", &viewport_len),
            ("visible_count", &visible_count),
            ("view_start_row", &view_start_row),
        ]));
        children.extend(browser_scrollbar_automation(
            scrollbar,
            &viewport_len,
            &visible_count,
        ));
    } else {
        table_node
            .metadata
            .insert(String::from("scrollbar_visible"), String::from("false"));
    }
    table_node.children = children;
    table_node
}

fn build_browser_pill_editor_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> Option<AutomationNodeSnapshot> {
    let rect = browser_pill_editor_panel_rect(layout.browser_rows, style.sizing, model)?;
    let sidebar = &model.browser.pill_editor;
    let option_pill_labels = sidebar
        .option_pills
        .iter()
        .map(|pill| pill.label.as_str())
        .collect::<Vec<_>>()
        .join("|");
    let mut children = vec![
        simple_node(
            "browser.pill_editor.input",
            AutomationRole::SearchField,
            Some(sidebar.input_placeholder.clone()),
            rect,
            Some(sidebar.input_value.clone()),
            true,
            false,
            vec![
                String::from("focus_browser_pill_editor_input"),
                String::from("set_browser_pill_editor_input"),
                String::from("commit_browser_pill_editor_input"),
            ],
        ),
        browser_pill_editor_pill_node(
            "browser.pill_editor.exclusive.0",
            &sidebar.exclusive_pills[0],
            rect,
            vec![String::from("set_browser_sidebar_looped")],
        ),
        browser_pill_editor_pill_node(
            "browser.pill_editor.exclusive.1",
            &sidebar.exclusive_pills[1],
            rect,
            vec![String::from("set_browser_sidebar_looped")],
        ),
    ];
    children.extend(sidebar.option_pills.iter().map(|pill| {
        browser_pill_editor_pill_node(
            format!("browser.pill_editor.option.{}", slug(&pill.label)),
            pill,
            rect,
            vec![String::from("toggle_browser_pill_option")],
        )
    }));
    if let Some(pill) = sidebar.create_pill.as_ref() {
        children.push(browser_pill_editor_pill_node(
            format!("browser.pill_editor.create.{}", slug(&pill.id)),
            pill,
            rect,
            vec![String::from("toggle_browser_pill_option")],
        ));
    }
    let mut node = simple_node(
        "browser.pill_editor",
        AutomationRole::Panel,
        Some(String::from("Pill editor")),
        rect,
        Some(sidebar.header_label.clone()),
        true,
        false,
        Vec::new(),
    );
    node.metadata = metadata(&[
        ("selected_count", &sidebar.selected_count.to_string()),
        (
            "primary_action_enabled",
            bool_text(sidebar.primary_action_enabled),
        ),
        ("option_pill_labels", &option_pill_labels),
    ]);
    node.children = children;
    Some(node)
}

fn browser_pill_editor_pill_node(
    id: impl Into<String>,
    pill: &crate::compat_app_contract::BrowserPillModel,
    rect: Rect,
    available_actions: Vec<String>,
) -> AutomationNodeSnapshot {
    let state = match pill.state {
        BrowserPillState::Off => "off",
        BrowserPillState::On => "on",
        BrowserPillState::Mixed => "mixed",
    };
    let mut node = simple_node(
        id,
        AutomationRole::Button,
        Some(pill.label.clone()),
        rect,
        Some(state.to_string()),
        true,
        pill.state == BrowserPillState::On,
        available_actions,
    );
    node.metadata = metadata(&[("pill_state", state), ("pill_id", pill.id.as_str())]);
    node
}

fn browser_scrollbar_automation(
    scrollbar: BrowserScrollbarLayout,
    viewport_len: &str,
    visible_count: &str,
) -> [AutomationNodeSnapshot; 2] {
    let track_metadata = metadata(&[
        ("viewport_len", viewport_len),
        ("visible_count", visible_count),
        ("part", "track"),
    ]);
    let thumb_metadata = metadata(&[
        ("viewport_len", viewport_len),
        ("visible_count", visible_count),
        ("part", "thumb"),
    ]);
    [
        AutomationNodeSnapshot {
            id: node_id("browser.scrollbar.track"),
            role: AutomationRole::Slider,
            label: Some(String::from("Browser scrollbar track")),
            bounds: bounds(scrollbar.track),
            value: None,
            enabled: true,
            selected: false,
            available_actions: vec![String::from("set_browser_view_start")],
            metadata: track_metadata,
            children: Vec::new(),
        },
        AutomationNodeSnapshot {
            id: node_id("browser.scrollbar.thumb"),
            role: AutomationRole::Slider,
            label: Some(String::from("Browser scrollbar thumb")),
            bounds: bounds(scrollbar.thumb),
            value: None,
            enabled: true,
            selected: false,
            available_actions: vec![String::from("set_browser_view_start")],
            metadata: thumb_metadata,
            children: Vec::new(),
        },
    ]
}

fn map_canvas_automation(
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let canvas = compute_browser_map_canvas_rect(layout.browser_rows, style.sizing);
    let mut map_node = simple_node(
        "browser.map_canvas",
        AutomationRole::SpatialCanvas,
        Some(model.browser_chrome.map_tab_label.clone()),
        canvas,
        Some(model.map.summary.clone()),
        true,
        true,
        vec![String::from("focus_spatial_content_item")],
    );
    map_node.children = model
        .map
        .points
        .iter()
        .map(|point| AutomationNodeSnapshot {
            id: node_id(format!("browser.map.point.{}", point.id)),
            role: AutomationRole::SpatialPoint,
            label: Some(String::from(point.id.as_ref())),
            bounds: bounds(circle_rect(
                compute_browser_map_point_center(canvas, point.x_milli, point.y_milli),
                10.0,
            )),
            value: None,
            enabled: true,
            selected: model.map.selected_item_id.as_deref() == Some(point.id.as_ref()),
            available_actions: vec![String::from("focus_spatial_content_item")],
            metadata: metadata(&[
                ("x_milli", &point.x_milli.to_string()),
                ("y_milli", &point.y_milli.to_string()),
            ]),
            children: Vec::new(),
        })
        .collect();
    map_node
}
