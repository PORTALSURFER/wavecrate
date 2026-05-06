//! Sidebar automation snapshot builders.

use super::helpers::{action_slug, bool_text, bounds, metadata, node_id, simple_node, slug};
use super::*;
use crate::compat_app_contract::{
    AutomationRole, BrowserPillState, FolderPaneIdModel, FolderPaneModel,
};

/// Build semantic automation for the sources/sidebar panel.
pub(super) fn build_sidebar_automation(
    shell: &mut NativeShellState,
    layout: &ShellLayout,
    model: &AppModel,
    style: &StyleTokens,
) -> AutomationNodeSnapshot {
    let source_rows = shell.cached_source_rows(layout, style, model).to_vec();
    let sections = sidebar_sections(layout, style, model);
    let mut children = Vec::new();
    let workspace = sidebar_workspace_sections(layout, style);
    children.push(simple_node(
        "sources.library",
        AutomationRole::Group,
        Some(String::from("Library")),
        layout.sidebar_header,
        Some(model.sources.header.clone()),
        true,
        false,
        vec![String::from("focus_sources_panel")],
    ));
    if let Some(rect) = source_add_button_rect(layout.sidebar_header, style.sizing) {
        children.push(simple_node(
            "sources.add_button",
            AutomationRole::Button,
            Some(String::from("Add source")),
            rect,
            None,
            true,
            false,
            vec![String::from("open_add_source_dialog")],
        ));
    }
    for button in source_action_buttons(layout, style, model) {
        children.push(simple_node(
            format!("sources.action.{}", slug(button.label)),
            AutomationRole::Button,
            Some(String::from(button.label)),
            button.rect,
            None,
            button.enabled,
            false,
            vec![action_slug(&button.action)],
        ));
    }
    let pane = model.sources.active_folder_pane;
    let pane_model = model.sources.folder_pane(pane);
    children.push(source_list_group(
        pane,
        sections.source_rows(pane),
        source_rows,
        &model.sources.rows,
        model.focus_context == crate::compat_app_contract::FocusContextModel::NavigationList,
    ));
    children.push(folder_browser_group(
        sections.folder_header(pane),
        sections.tree_rows(pane),
        shell.cached_tree_rows(layout, style, model, pane).to_vec(),
        &pane_model.tree_rows,
        pane_model,
        style,
        model.focus_context == crate::compat_app_contract::FocusContextModel::NavigationTree,
    ));
    children.push(tags_group(workspace.tags, model));
    children.push(filters_group(workspace.filters, model));
    AutomationNodeSnapshot {
        id: node_id("sources.panel"),
        role: AutomationRole::Panel,
        label: Some(String::from("Sources")),
        bounds: bounds(layout.sidebar),
        value: Some(model.sources.header.clone()),
        enabled: true,
        selected: false,
        available_actions: vec![String::from("focus_sources_panel")],
        metadata: metadata(&[
            ("source_search", model.sources.search_query.as_str()),
            (
                "active_folder_search",
                model
                    .sources
                    .active_folder_pane_model()
                    .tree_search_query
                    .as_str(),
            ),
        ]),
        children,
    }
}

/// Build automation nodes for the sidebar tag editor.
fn tags_group(rect: Rect, model: &AppModel) -> AutomationNodeSnapshot {
    let sidebar = model.browser.pill_editor();
    let mut children = Vec::new();
    children.push(simple_node(
        "sources.tags.input",
        AutomationRole::SearchField,
        Some(String::from("Add tag")),
        sidebar_tag_input_rect_for_automation(rect),
        Some(sidebar.input_value.clone()),
        true,
        false,
        vec![
            String::from("focus_browser_pill_editor_input"),
            String::from("set_browser_pill_editor_input"),
            String::from("commit_browser_pill_editor_input"),
        ],
    ));
    children.extend(
        sidebar
            .option_pills
            .iter()
            .enumerate()
            .map(|(index, pill)| AutomationNodeSnapshot {
                id: node_id(format!("sources.tags.suggestion.{index}")),
                role: AutomationRole::Button,
                label: Some(pill.label.clone()),
                bounds: bounds(rect),
                value: Some(format!("{:?}", pill.state)),
                enabled: true,
                selected: !matches!(pill.state, BrowserPillState::Off),
                available_actions: vec![String::from("toggle_browser_pill_option")],
                metadata: metadata(&[("pill_id", pill.id.as_str())]),
                children: Vec::new(),
            }),
    );
    if let Some(pill) = sidebar.create_pill.as_ref() {
        children.push(AutomationNodeSnapshot {
            id: node_id(format!("sources.tags.create_tag.{}", slug(&pill.id))),
            role: AutomationRole::Button,
            label: Some(pill.label.clone()),
            bounds: bounds(rect),
            value: Some(format!("{:?}", pill.state)),
            enabled: true,
            selected: false,
            available_actions: vec![String::from("commit_browser_pill_editor_input")],
            metadata: metadata(&[("pill_id", pill.id.as_str())]),
            children: Vec::new(),
        });
    }
    let option_pill_labels = sidebar
        .option_pills
        .iter()
        .map(|pill| pill.label.as_str())
        .collect::<Vec<_>>()
        .join("|");
    AutomationNodeSnapshot {
        id: node_id("sources.tags"),
        role: AutomationRole::Group,
        label: Some(String::from("Tags")),
        bounds: bounds(rect),
        value: Some(sidebar.header_label.clone()),
        enabled: true,
        selected: false,
        available_actions: vec![String::from("focus_browser_pill_editor_input")],
        metadata: metadata(&[
            ("selected_count", &sidebar.selected_count.to_string()),
            ("normal_tag_labels", option_pill_labels.as_str()),
        ]),
        children,
    }
}

/// Build automation nodes for the sidebar browser filters.
fn filters_group(rect: Rect, model: &AppModel) -> AutomationNodeSnapshot {
    let rows = ["format", "bit_depth", "channels", "bpm", "key", "rating"];
    let mut children: Vec<_> = rows
        .into_iter()
        .map(|name| {
            let (value, actions) = match name {
                "format" => (
                    sidebar_option_summary(model.sidebar_filters.formats.len(), "WAV"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "bit_depth" => (
                    sidebar_option_summary(model.sidebar_filters.bit_depths.len(), "Unavailable"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "channels" => (
                    sidebar_option_summary(model.sidebar_filters.channels.len(), "Unavailable"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "bpm" => (
                    sidebar_bpm_summary(model),
                    vec![
                        String::from("toggle_browser_sidebar_filter"),
                        String::from("clear_browser_sidebar_filter"),
                    ],
                ),
                "key" => (
                    sidebar_option_summary(model.sidebar_filters.keys.len(), "Unknown"),
                    vec![String::from("toggle_browser_sidebar_filter")],
                ),
                "rating" => (
                    rating_filter_summary(model),
                    vec![String::from("toggle_browser_rating_filter")],
                ),
                _ => (String::new(), Vec::new()),
            };
            simple_node(
                format!("sources.filters.{name}"),
                AutomationRole::Button,
                Some(name.replace('_', " ")),
                rect,
                Some(value),
                true,
                false,
                actions,
            )
        })
        .collect();
    children.push(simple_node(
        "sources.filters.marked",
        AutomationRole::Button,
        Some(String::from("Marked")),
        rect,
        Some(bool_text(model.browser.marked_filter_active).to_string()),
        true,
        model.browser.marked_filter_active,
        vec![String::from("toggle_browser_marked_filter")],
    ));
    for (slug, active) in [
        ("never", model.browser.active_recency_filters[0]),
        ("month", model.browser.active_recency_filters[1]),
        ("week", model.browser.active_recency_filters[2]),
    ] {
        children.push(simple_node(
            format!("sources.filters.playback_age.{slug}"),
            AutomationRole::Button,
            Some(format!("Playback age {slug}")),
            rect,
            Some(bool_text(active).to_string()),
            true,
            active,
            vec![String::from("toggle_browser_playback_age_filter")],
        ));
    }
    children.push(simple_node(
        "sources.filters.tag_named",
        AutomationRole::Button,
        Some(String::from("Tag-derived names")),
        rect,
        Some(if model.browser.derived_label_filter_negated {
            String::from("not tag-derived")
        } else if model.browser.derived_label_filter_active {
            String::from("tag-derived")
        } else {
            String::from("any")
        }),
        true,
        model.browser.derived_label_filter_active,
        vec![String::from("toggle_browser_tag_named_filter")],
    ));
    AutomationNodeSnapshot {
        id: node_id("sources.filters"),
        role: AutomationRole::Group,
        label: Some(String::from("Filters")),
        bounds: bounds(rect),
        value: None,
        enabled: true,
        selected: false,
        available_actions: vec![String::from("focus_browser_panel")],
        metadata: metadata(&[("placement", "left_sidebar")]),
        children,
    }
}

/// Summarize single-option sidebar facets for automation.
fn sidebar_option_summary(active_count: usize, label: &str) -> String {
    if active_count == 0 {
        String::from("Any")
    } else {
        label.to_string()
    }
}

/// Summarize active BPM sidebar facets for automation.
fn sidebar_bpm_summary(model: &AppModel) -> String {
    if model.sidebar_filters.bpms.is_empty() {
        String::from("Any")
    } else {
        model
            .sidebar_filters
            .bpms
            .iter()
            .map(|facet| format!("{facet:?}"))
            .collect::<Vec<_>>()
            .join("|")
    }
}

/// Summarize the active rating filters for automation.
fn rating_filter_summary(model: &AppModel) -> String {
    let active = model
        .browser
        .active_rating_filters
        .iter()
        .filter(|active| **active)
        .count();
    if active == 0 {
        String::from("Any")
    } else {
        format!("{active} active")
    }
}

/// Return the sidebar tag input bounds used by automation snapshots.
fn sidebar_tag_input_rect_for_automation(rect: Rect) -> Rect {
    let pad = 6.0;
    let height = 18.0;
    Rect::from_min_max(
        Point::new(
            rect.min.x + pad,
            (rect.max.y - pad - height).max(rect.min.y + pad),
        ),
        Point::new(rect.max.x - pad, rect.max.y - pad),
    )
}

fn source_list_group(
    pane: FolderPaneIdModel,
    rect: Rect,
    source_rows: Vec<CachedSourceRow>,
    rows: &[crate::compat_app_contract::SourceRowModel],
    selected: bool,
) -> AutomationNodeSnapshot {
    let row_count = rows.len().to_string();
    let children = source_rows
        .into_iter()
        .filter_map(|rendered_row| {
            rows.get(rendered_row.row_index)
                .map(|row| (rendered_row.row_index, rendered_row.rect, row))
        })
        .map(|(index, rect, row)| AutomationNodeSnapshot {
            id: node_id(format!("sources.source_row.{index}")),
            role: AutomationRole::Row,
            label: Some(row.label.clone()),
            bounds: bounds(rect),
            value: (!row.detail.is_empty()).then(|| row.detail.clone()),
            enabled: true,
            selected: source_row_selected(row, pane),
            available_actions: vec![
                String::from("select_source_row"),
                String::from("reload_source_row"),
                String::from("hard_sync_source_row"),
                String::from("open_source_folder_row"),
                String::from("remove_source_row"),
            ],
            metadata: metadata(&[
                ("detail", row.detail.as_str()),
                ("missing", bool_text(row.missing)),
            ]),
            children: Vec::new(),
        })
        .collect();
    AutomationNodeSnapshot {
        id: node_id("sources.source_list"),
        role: AutomationRole::Group,
        label: Some(String::from("source list")),
        bounds: bounds(rect),
        value: None,
        enabled: true,
        selected,
        available_actions: vec![String::from("focus_sources_panel")],
        metadata: metadata(&[("row_count", &row_count)]),
        children,
    }
}

fn source_row_selected(
    row: &crate::compat_app_contract::SourceRowModel,
    pane: FolderPaneIdModel,
) -> bool {
    match pane {
        FolderPaneIdModel::Upper => row.assigned_to_upper_pane,
        FolderPaneIdModel::Lower => row.assigned_to_lower_pane,
    }
}

fn folder_browser_group(
    header_rect: Rect,
    tree_rows_band: Rect,
    tree_rows: Vec<CachedFolderRow>,
    rows: &[crate::compat_app_contract::FolderRowModel],
    pane_model: &FolderPaneModel,
    style: &StyleTokens,
    selected: bool,
) -> AutomationNodeSnapshot {
    let row_count = rows.len().to_string();
    let mut children = Vec::new();
    if let Some(toggle_button) = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        pane_model.recovery.in_progress,
        pane_model.recovery.entry_count,
        pane_model.show_all_items,
        pane_model.can_toggle_show_all_items,
        pane_model.flattened_view,
        pane_model.can_toggle_flattened_view,
    )
    .visibility_toggle_button
    {
        children.push(simple_node(
            "sources.folder_visibility_toggle",
            AutomationRole::Button,
            Some(String::from("Folder visibility")),
            toggle_button.rect,
            Some(if toggle_button.active {
                String::from("All folders")
            } else {
                String::from("WAV folders")
            }),
            toggle_button.enabled,
            toggle_button.active,
            vec![String::from("toggle_show_all_folders")],
        ));
    }
    if let Some(toggle_button) = compute_sidebar_folder_header_layout(
        header_rect,
        style.sizing,
        pane_model.recovery.in_progress,
        pane_model.recovery.entry_count,
        pane_model.show_all_items,
        pane_model.can_toggle_show_all_items,
        pane_model.flattened_view,
        pane_model.can_toggle_flattened_view,
    )
    .flatten_toggle_button
    {
        children.push(simple_node(
            "sources.folder_flatten_toggle",
            AutomationRole::Button,
            Some(String::from("Flattened view")),
            toggle_button.rect,
            Some(if toggle_button.active {
                String::from("All descendants")
            } else {
                String::from("Direct only")
            }),
            toggle_button.enabled,
            toggle_button.active,
            vec![String::from("toggle_folder_flattened_view")],
        ));
    }
    children.extend(
        tree_rows
            .into_iter()
            .filter_map(|rendered_row| {
                rows.get(rendered_row.row_index)
                    .map(|row| (rendered_row.row_index, rendered_row.rect, row))
            })
            .map(|(row_index, rect, row)| {
                let (role, label, value, available_actions) = if matches!(
                    row.kind,
                    crate::compat_app_contract::FolderRowKind::CreateDraft
                        | crate::compat_app_contract::FolderRowKind::RenameDraft
                ) {
                    (
                        AutomationRole::SearchField,
                        Some(
                            if row.kind == crate::compat_app_contract::FolderRowKind::RenameDraft {
                                String::from("Rename folder")
                            } else {
                                String::from("New folder")
                            },
                        ),
                        row.input_value.clone(),
                        vec![
                            String::from("focus_folder_create_input"),
                            String::from("set_folder_create_input"),
                            String::from("confirm_folder_create"),
                            String::from("cancel_folder_create"),
                        ],
                    )
                } else {
                    let mut available_actions = vec![
                        String::from("focus_folder_row"),
                        String::from("activate_folder_row"),
                        String::from("start_new_folder_at_folder_row"),
                    ];
                    if row.has_children && !row.is_root {
                        available_actions.push(String::from("toggle_folder_row_expanded"));
                    }
                    (
                        AutomationRole::Row,
                        Some(row.label.clone()),
                        (!row.detail.is_empty()).then(|| row.detail.clone()),
                        available_actions,
                    )
                };
                AutomationNodeSnapshot {
                    id: node_id(format!("sources.folder_row.{row_index}")),
                    role,
                    label,
                    bounds: bounds(rect),
                    value,
                    enabled: true,
                    selected: row.selected || row.focused || row.input_focused,
                    available_actions,
                    metadata: metadata(&[
                        ("depth", &row.depth.to_string()),
                        ("focused", bool_text(row.focused)),
                        ("root", bool_text(row.is_root)),
                        ("expanded", bool_text(row.expanded)),
                        (
                            "kind",
                            match row.kind {
                                crate::compat_app_contract::FolderRowKind::CreateDraft => {
                                    "create_draft"
                                }
                                crate::compat_app_contract::FolderRowKind::RenameDraft => {
                                    "rename_draft"
                                }
                                crate::compat_app_contract::FolderRowKind::Existing => "existing",
                            },
                        ),
                        ("input_error", row.input_error.as_deref().unwrap_or("")),
                        ("select_all_on_focus", bool_text(row.select_all_on_focus)),
                    ]),
                    children: Vec::new(),
                }
            }),
    );
    AutomationNodeSnapshot {
        id: node_id("sources.folder_browser"),
        role: AutomationRole::Group,
        label: Some(String::from("folder browser")),
        bounds: bounds(union_rect(header_rect, tree_rows_band)),
        value: Some(pane_model.item_label.clone()),
        enabled: true,
        selected,
        available_actions: vec![String::from("focus_folder_panel")],
        metadata: metadata(&[
            ("row_count", &row_count),
            ("item_detail", pane_model.item_detail.as_str()),
            (
                "visibility",
                if pane_model.show_all_items {
                    "all_folders"
                } else {
                    "wav_folders"
                },
            ),
            (
                "flattened_view",
                if pane_model.flattened_view {
                    "all_descendants"
                } else {
                    "direct_only"
                },
            ),
        ]),
        children,
    }
}

fn union_rect(first: Rect, second: Rect) -> Rect {
    Rect::from_min_max(
        Point::new(first.min.x.min(second.min.x), first.min.y.min(second.min.y)),
        Point::new(first.max.x.max(second.max.x), first.max.y.max(second.max.y)),
    )
}
