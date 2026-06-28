use crate::app_core::actions::{
    NativeAppModel, NativeAutomationBounds, NativeAutomationNodeId, NativeAutomationNodeSnapshot,
    NativeAutomationRole, NativeGuiAutomationSnapshot, NativeUpdateStatusModel,
};
use std::collections::BTreeMap;

/// Capture a deterministic GUI automation snapshot without launching the native host.
pub fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let viewport_width = viewport[0].max(0.0).round() as u32;
    let viewport_height = viewport[1].max(0.0).round() as u32;
    NativeGuiAutomationSnapshot {
        schema_version: 1,
        viewport_width,
        viewport_height,
        root: automation_node(
            "shell.root",
            NativeAutomationRole::Root,
            Some(format!("{} shell", model.title)),
            bounds(0.0, 0.0, viewport[0], viewport[1]),
            None,
            false,
            shell_children(viewport, model),
        ),
    }
}

fn shell_children(viewport: [f32; 2], model: &NativeAppModel) -> Vec<NativeAutomationNodeSnapshot> {
    let width = viewport[0].max(0.0);
    let height = viewport[1].max(0.0);
    let top_height = 40.0;
    let status_height = 28.0;
    let sidebar_width = 264.0_f32.min(width * 0.4);
    let content_width = (width - sidebar_width).max(0.0);
    let content_height = (height - top_height - status_height).max(0.0);
    let waveform_height = (content_height * 0.44).max(0.0);

    let mut children = vec![
        top_bar(width, top_height, model),
        sources_panel(sidebar_width, top_height, content_height, model),
        waveform_panel(
            sidebar_width,
            top_height,
            content_width,
            waveform_height,
            model,
        ),
        browser_panel(
            sidebar_width,
            top_height + waveform_height,
            content_width,
            (content_height - waveform_height).max(0.0),
            model,
        ),
        status_bar(
            width,
            (height - status_height).max(0.0),
            status_height,
            model,
        ),
    ];
    if model.options_panel.visible {
        children.push(options_overlay(width, height, model));
    }
    if model.confirm_prompt.visible {
        children.push(prompt_overlay(width, height, model));
    }
    children
}

/// Build top-bar automation targets for transport, volume, options, and update controls.
fn top_bar(width: f32, height: f32, model: &NativeAppModel) -> NativeAutomationNodeSnapshot {
    let mut children = vec![
        with_actions(
            automation_node(
                "shell.top_bar.volume_slider",
                NativeAutomationRole::Slider,
                Some(String::from("Volume")),
                bounds((width - 252.0).max(0.0), 8.0, 112.0, 24.0),
                Some(format!("{:.3}", model.volume.clamp(0.0, 1.0))),
                false,
                Vec::new(),
            ),
            &["set_volume", "commit_volume_setting"],
        ),
        with_actions(
            automation_node(
                "shell.top_bar.options_button",
                NativeAutomationRole::Button,
                Some(String::from("Options")),
                bounds((width - 56.0).max(0.0), 4.0, 40.0, 32.0),
                None,
                model.options_panel.visible,
                Vec::new(),
            ),
            if model.options_panel.visible {
                &["close_options_panel"]
            } else {
                &["open_options_menu"]
            },
        ),
    ];
    if update_panel_is_visible(model) {
        children.push(update_panel_node(
            (width - 392.0).max(0.0),
            4.0,
            128.0,
            32.0,
            model,
        ));
    }

    automation_node(
        "shell.top_bar",
        NativeAutomationRole::Panel,
        Some(String::from("Top bar")),
        bounds(0.0, 0.0, width, height),
        None,
        false,
        children,
    )
}

/// Build source-list, folder-browser, and tag-library automation targets.
fn sources_panel(
    width: f32,
    y: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let source_list_height = (height * 0.36).max(0.0);
    let folder_y = y + source_list_height;
    let folder_height = (height - source_list_height).max(0.0);
    let source_rows = model
        .sources
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            with_actions(
                automation_node(
                    format!("sources.source_row.{index}"),
                    NativeAutomationRole::Row,
                    Some(row.label.clone()),
                    bounds(0.0, y + 36.0 + index as f32 * 24.0, width, 24.0),
                    Some(row.detail.clone()),
                    model.sources.selected_row == Some(index),
                    Vec::new(),
                ),
                &[
                    "select_source_row",
                    "reload_source_row",
                    "hard_sync_source_row",
                    "open_source_folder_row",
                    "remove_source_row",
                ],
            )
        })
        .collect();
    let folder_rows = model
        .sources
        .tree_rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            with_actions(
                automation_node(
                    format!("sources.folder_row.{index}"),
                    NativeAutomationRole::Row,
                    Some(row.label.clone()),
                    bounds(0.0, folder_y + 72.0 + index as f32 * 23.0, width, 23.0),
                    Some(row.detail.clone()),
                    row.flags.selected,
                    Vec::new(),
                ),
                &[
                    "focus_folder_row",
                    "activate_folder_row",
                    "start_new_folder_at_folder_row",
                    "toggle_folder_row_expanded",
                ],
            )
        })
        .collect();

    with_actions(
        automation_node(
            "sources.panel",
            NativeAutomationRole::Panel,
            Some(String::from("Sources")),
            bounds(0.0, y, width, height),
            None,
            false,
            vec![
                with_actions(
                    automation_node(
                        "sources.add_button",
                        NativeAutomationRole::Button,
                        Some(String::from("Add source")),
                        bounds(8.0, y + 8.0, 32.0, 28.0),
                        None,
                        false,
                        Vec::new(),
                    ),
                    &["open_add_source_dialog"],
                ),
                with_actions(
                    automation_node(
                        "sources.source_list",
                        NativeAutomationRole::Table,
                        Some(String::from("Sources")),
                        bounds(0.0, y, width, source_list_height),
                        Some(format!("{} sources", model.sources.rows.len())),
                        false,
                        source_rows,
                    ),
                    &["focus_sources_panel"],
                ),
                with_actions(
                    automation_node(
                        "sources.folder_browser",
                        NativeAutomationRole::Table,
                        Some(String::from("Folders")),
                        bounds(0.0, folder_y, width, folder_height),
                        Some(format!(
                            "{} folders",
                            model.sources.upper_folder_pane.tree_rows.len()
                        )),
                        false,
                        folder_browser_children(width, folder_y, folder_height, folder_rows),
                    ),
                    &["focus_folder_panel"],
                ),
                tag_library_node(width, y + 48.0, model),
            ],
        ),
        &["focus_sources_panel"],
    )
}

fn browser_panel(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let table_height = (height - 96.0).max(0.0);
    let table_y = y + 72.0;
    let table = with_actions(
        automation_node(
            "browser.table",
            NativeAutomationRole::Table,
            Some(String::from("Samples")),
            bounds(x, table_y, width, table_height),
            Some(format!("{} rows", model.browser.visible_count)),
            false,
            model
                .browser
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    with_actions(
                        automation_node(
                            format!("browser.row.{index}"),
                            NativeAutomationRole::Row,
                            Some(row.label.to_string()),
                            bounds(x, table_y + 24.0 + index as f32 * 22.0, width, 22.0),
                            None,
                            model.browser.selected_visible_row == Some(index),
                            Vec::new(),
                        ),
                        &[
                            "focus_browser_row",
                            "toggle_browser_row_selection",
                            "commit_focused_browser_row",
                        ],
                    )
                })
                .collect(),
        ),
        &["focus_browser_panel", "set_browser_view_start"],
    );
    let mut children = vec![
        browser_tab_list(x, y, width, model),
        with_actions(
            automation_node(
                "browser.search_field",
                NativeAutomationRole::SearchField,
                Some(String::from("Search samples")),
                bounds(x + 16.0, y + 36.0, (width - 32.0).max(0.0), 28.0),
                Some(model.browser.search_query.clone()),
                false,
                Vec::new(),
            ),
            &["set_browser_search"],
        ),
        browser_filter_node(
            "sources.filters.rating",
            "Rating filter",
            x + 16.0,
            y + 8.0,
            model
                .browser
                .active_rating_filters
                .iter()
                .any(|active| *active),
            &["toggle_browser_rating_filter"],
        ),
        browser_filter_node(
            "sources.filters.playback_age.month",
            "Older than month",
            x + 220.0,
            y + 8.0,
            model
                .browser
                .active_playback_age_filters
                .get(1)
                .copied()
                .unwrap_or(false),
            &["toggle_browser_playback_age_filter"],
        ),
        with_actions(
            automation_node(
                "sources.tags.input",
                NativeAutomationRole::SearchField,
                Some(String::from("Tag input")),
                bounds((x + width - 224.0).max(x), y + 8.0, 208.0, 28.0),
                Some(model.browser.tag_sidebar.input_value.clone()),
                false,
                Vec::new(),
            ),
            &[
                "focus_browser_tag_sidebar_input",
                "set_browser_tag_sidebar_input",
                "commit_browser_tag_sidebar_input",
            ],
        ),
        table,
        with_actions(
            automation_node(
                "browser.scrollbar.thumb",
                NativeAutomationRole::Slider,
                Some(String::from("Browser scrollbar thumb")),
                bounds((x + width - 16.0).max(x), table_y + 24.0, 12.0, 48.0),
                Some(model.browser.view_start_row.to_string()),
                false,
                Vec::new(),
            ),
            &["set_browser_view_start"],
        ),
        with_actions(
            automation_node(
                "browser.scrollbar.track",
                NativeAutomationRole::Slider,
                Some(String::from("Browser scrollbar track")),
                bounds((x + width - 16.0).max(x), table_y, 12.0, table_height),
                Some(model.browser.view_start_row.to_string()),
                false,
                Vec::new(),
            ),
            &["set_browser_view_start"],
        ),
    ];
    if model.map.active || !model.map.points.is_empty() {
        children.push(map_canvas_node(x, table_y, width, table_height, model));
    }

    let mut panel = with_actions(
        automation_node(
            "browser.panel",
            NativeAutomationRole::Panel,
            Some(String::from("Browser")),
            bounds(x, y, width, height),
            None,
            false,
            children,
        ),
        &["focus_browser_panel"],
    );
    panel.metadata.insert(
        String::from("focused_sample_label"),
        model
            .browser
            .focused_sample_label
            .clone()
            .unwrap_or_default(),
    );
    panel.metadata.insert(
        String::from("selected_visible_row"),
        model
            .browser
            .selected_visible_row
            .map(|row| row.to_string())
            .unwrap_or_else(|| String::from("none")),
    );
    panel
}

/// Build waveform automation targets for toolbar actions, region gestures, and selections.
fn waveform_panel(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let region_bounds = bounds(
        x + 12.0,
        y + 28.0,
        (width - 24.0).max(0.0),
        (height - 40.0).max(0.0),
    );
    let mut region = with_actions(
        automation_node(
            "waveform.region",
            NativeAutomationRole::WaveformRegion,
            model.waveform.loaded_label.clone(),
            region_bounds,
            Some(
                model
                    .waveform
                    .loaded_label
                    .clone()
                    .unwrap_or_else(|| String::from("No sample loaded")),
            ),
            false,
            Vec::new(),
        ),
        &[
            "detect_waveform_silence_slices",
            "detect_waveform_exact_duplicate_slices",
            "clean_waveform_exact_duplicate_slices",
            "audition_waveform_duplicate_slice",
            "toggle_waveform_duplicate_slice_exemption",
            "move_waveform_slice_focus",
            "toggle_focused_waveform_slice_export_mark",
            "play_waveform_at_precise",
            "clear_waveform_selections",
            "seek_waveform_precise",
            "set_waveform_cursor_precise",
            "set_waveform_selection_range",
            "zoom_waveform",
            "set_waveform_view_center",
        ],
    );
    region.metadata.insert(
        String::from("cursor_milli"),
        model
            .waveform
            .cursor_milli
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    region.metadata.insert(
        String::from("zoom_label"),
        model.waveform.zoom_label.clone().unwrap_or_default(),
    );
    let mut children = vec![
        with_actions(
            automation_node(
                "waveform.toolbar.play",
                NativeAutomationRole::Button,
                Some(String::from("Play")),
                bounds(x + 12.0, y + 4.0, 48.0, 24.0),
                None,
                model.transport_running,
                Vec::new(),
            ),
            &["toggle_transport"],
        ),
        region,
    ];
    if model.waveform.selection_milli.is_some() {
        let mut selection = with_actions(
            automation_node(
                "waveform.selection",
                NativeAutomationRole::WaveformRegion,
                Some(String::from("Waveform selection")),
                region_bounds,
                None,
                true,
                Vec::new(),
            ),
            &["clear_waveform_selection"],
        );
        selection.metadata.insert(
            String::from("selection_micros"),
            normalized_range_micros(model.waveform.selection_milli.as_ref()),
        );
        children.push(selection);
    }
    if model.waveform.edit_selection_milli.is_some() {
        let mut edit_selection = with_actions(
            automation_node(
                "waveform.edit_selection",
                NativeAutomationRole::WaveformRegion,
                Some(String::from("Waveform edit selection")),
                region_bounds,
                None,
                true,
                Vec::new(),
            ),
            &["clear_waveform_edit_selection"],
        );
        edit_selection.metadata.insert(
            String::from("selection_micros"),
            normalized_range_micros(model.waveform.edit_selection_milli.as_ref()),
        );
        children.push(edit_selection);
    }

    with_actions(
        automation_node(
            "waveform.panel",
            NativeAutomationRole::Panel,
            Some(String::from("Waveform")),
            bounds(x, y, width, height),
            None,
            false,
            children,
        ),
        &["focus_waveform_panel"],
    )
}

/// Build the folder-browser control and row targets for the sources panel.
fn folder_browser_children(
    width: f32,
    y: f32,
    height: f32,
    mut folder_rows: Vec<NativeAutomationNodeSnapshot>,
) -> Vec<NativeAutomationNodeSnapshot> {
    let mut children = vec![
        with_actions(
            automation_node(
                "sources.folder_visibility_toggle",
                NativeAutomationRole::Button,
                Some(String::from("Show all folders")),
                bounds(8.0, y + 8.0, (width * 0.5 - 12.0).max(0.0), 28.0),
                None,
                false,
                Vec::new(),
            ),
            &["toggle_show_all_folders"],
        ),
        with_actions(
            automation_node(
                "sources.folder_flatten_toggle",
                NativeAutomationRole::Button,
                Some(String::from("Flatten folders")),
                bounds(width * 0.5, y + 8.0, (width * 0.5 - 8.0).max(0.0), 28.0),
                None,
                false,
                Vec::new(),
            ),
            &["toggle_folder_flattened_view"],
        ),
    ];
    for row in &mut folder_rows {
        row.bounds.width = row.bounds.width.min(width);
        row.bounds.y = row.bounds.y.min(y + height);
    }
    children.extend(folder_rows);
    children
}

/// Build browser tab targets for switching between sample and map views.
fn browser_tab_list(
    x: f32,
    y: f32,
    width: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    automation_node(
        "browser.tabs",
        NativeAutomationRole::TabList,
        Some(String::from("Browser tabs")),
        bounds(x + 16.0, y + 4.0, width.min(240.0), 28.0),
        None,
        false,
        vec![
            with_actions(
                automation_node(
                    "browser.tab.samples",
                    NativeAutomationRole::Tab,
                    Some(model.browser_chrome.samples_tab_label.clone()),
                    bounds(x + 16.0, y + 4.0, 108.0, 28.0),
                    None,
                    !model.map.active,
                    Vec::new(),
                ),
                &["set_browser_tab"],
            ),
            with_actions(
                automation_node(
                    "browser.tab.map",
                    NativeAutomationRole::Tab,
                    Some(model.browser_chrome.map_tab_label.clone()),
                    bounds(x + 128.0, y + 4.0, 112.0, 28.0),
                    None,
                    model.map.active,
                    Vec::new(),
                ),
                &["set_browser_tab"],
            ),
        ],
    )
}

/// Build tag-sidebar automation targets and metadata from the projected tag model.
fn tag_library_node(width: f32, y: f32, model: &NativeAppModel) -> NativeAutomationNodeSnapshot {
    let sidebar = &model.browser.tag_sidebar;
    let mut children: Vec<_> = sidebar
        .option_pills
        .iter()
        .enumerate()
        .map(|(index, pill)| {
            with_actions(
                automation_node(
                    format!("sources.tags.suggestion.{index}"),
                    NativeAutomationRole::Button,
                    Some(pill.label.clone()),
                    bounds(8.0, y + 42.0 + index as f32 * 24.0, width - 16.0, 22.0),
                    None,
                    false,
                    Vec::new(),
                ),
                &["toggle_browser_sidebar_normal_tag"],
            )
        })
        .collect();
    if let Some(create_pill) = sidebar.create_pill.as_ref() {
        children.push(with_actions(
            automation_node(
                format!(
                    "sources.tags.create_tag.{}",
                    tag_node_suffix(&create_tag_value(&create_pill.label))
                ),
                NativeAutomationRole::Button,
                Some(create_pill.label.clone()),
                bounds(
                    8.0,
                    y + 42.0 + children.len() as f32 * 24.0,
                    width - 16.0,
                    22.0,
                ),
                None,
                false,
                Vec::new(),
            ),
            &["commit_browser_tag_sidebar_input"],
        ));
    }

    let mut node = automation_node(
        "sources.tags",
        NativeAutomationRole::Panel,
        Some(String::from("Tags")),
        bounds(0.0, y, width, 132.0),
        Some(sidebar.input_value.clone()),
        false,
        children,
    );
    node.metadata.insert(
        String::from("normal_tag_labels"),
        sidebar
            .option_pills
            .iter()
            .map(|pill| pill.label.as_str())
            .collect::<Vec<_>>()
            .join("|"),
    );
    node.metadata.insert(
        String::from("accepted_tag_labels"),
        sidebar
            .accepted_pills
            .iter()
            .map(|pill| pill.label.as_str())
            .collect::<Vec<_>>()
            .join("|"),
    );
    node
}

/// Convert a tag label into a stable automation ID suffix.
fn tag_node_suffix(label: &str) -> String {
    let mut suffix = String::new();
    let mut last_was_separator = false;
    for character in label.chars() {
        if character.is_ascii_alphanumeric() {
            suffix.push(character.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            suffix.push('_');
            last_was_separator = true;
        }
    }
    suffix.trim_matches('_').to_owned()
}

/// Extract the raw tag value from the projected create-tag pill label.
fn create_tag_value(label: &str) -> String {
    label
        .strip_prefix("Create \"")
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(label)
        .to_owned()
}

/// Build one browser filter target with its action metadata.
fn browser_filter_node(
    id: &'static str,
    label: &'static str,
    x: f32,
    y: f32,
    selected: bool,
    actions: &[&str],
) -> NativeAutomationNodeSnapshot {
    with_actions(
        automation_node(
            id,
            NativeAutomationRole::Button,
            Some(String::from(label)),
            bounds(x, y, 96.0, 28.0),
            None,
            selected,
            Vec::new(),
        ),
        actions,
    )
}

/// Build map canvas and point automation targets from the projected map model.
fn map_canvas_node(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let point_nodes = model
        .map
        .points
        .iter()
        .map(|point| {
            let point_x = x + width * (f32::from(point.x_milli.min(1000)) / 1000.0);
            let point_y = y + height * (f32::from(point.y_milli.min(1000)) / 1000.0);
            with_actions(
                automation_node(
                    format!("browser.map.point.{}", point.id),
                    NativeAutomationRole::MapPoint,
                    Some(point.id.to_string()),
                    bounds(point_x - 6.0, point_y - 6.0, 12.0, 12.0),
                    point.cluster_id.map(|cluster| format!("cluster {cluster}")),
                    model.map.selected_item_id.as_deref() == Some(point.id.as_ref()),
                    Vec::new(),
                ),
                &["focus_map_sample"],
            )
        })
        .collect();
    automation_node(
        "browser.map_canvas",
        NativeAutomationRole::MapCanvas,
        Some(model.map.summary.clone()),
        bounds(x, y, width, height),
        Some(model.map.legend_label.clone()),
        false,
        point_nodes,
    )
}

/// Build the options overlay target when the options panel is visible.
fn options_overlay(
    width: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    with_actions(
        automation_node(
            "overlay.options_panel",
            NativeAutomationRole::Dialog,
            Some(String::from("Options")),
            centered_bounds(width, height, 520.0, 420.0),
            Some(model.options_panel.default_identifier.clone()),
            false,
            vec![with_actions(
                automation_node(
                    "overlay.options_panel.close",
                    NativeAutomationRole::Button,
                    Some(String::from("Close options")),
                    bounds(
                        (width + 520.0) * 0.5 - 48.0,
                        (height - 420.0) * 0.5 + 12.0,
                        36.0,
                        28.0,
                    ),
                    None,
                    false,
                    Vec::new(),
                ),
                &["close_options_panel"],
            )],
        ),
        &["close_options_panel"],
    )
}

/// Build confirmation prompt automation targets, including optional input.
fn prompt_overlay(width: f32, height: f32, model: &NativeAppModel) -> NativeAutomationNodeSnapshot {
    let mut children = Vec::new();
    if model.confirm_prompt.input_value.is_some()
        || model.confirm_prompt.input_placeholder.is_some()
    {
        children.push(with_actions(
            automation_node(
                "overlay.prompt.input",
                NativeAutomationRole::SearchField,
                model.confirm_prompt.input_placeholder.clone(),
                centered_bounds(width, height, 360.0, 32.0),
                model.confirm_prompt.input_value.clone(),
                false,
                Vec::new(),
            ),
            &["set_prompt_input"],
        ));
    }
    children.push(with_actions(
        automation_node(
            "overlay.prompt.confirm",
            NativeAutomationRole::Button,
            Some(model.confirm_prompt.confirm_label.clone()),
            bounds(width * 0.5 - 156.0, height * 0.5 + 92.0, 144.0, 32.0),
            None,
            false,
            Vec::new(),
        ),
        &["confirm_prompt"],
    ));
    children.push(with_actions(
        automation_node(
            "overlay.prompt.cancel",
            NativeAutomationRole::Button,
            Some(model.confirm_prompt.cancel_label.clone()),
            bounds(width * 0.5 + 12.0, height * 0.5 + 92.0, 144.0, 32.0),
            None,
            false,
            Vec::new(),
        ),
        &["cancel_prompt"],
    ));
    automation_node(
        "overlay.prompt",
        NativeAutomationRole::Dialog,
        Some(model.confirm_prompt.title.clone()),
        centered_bounds(width, height, 520.0, 280.0),
        Some(model.confirm_prompt.message.clone()),
        false,
        children,
    )
}

/// Return whether update status should be exposed in the top-bar automation tree.
fn update_panel_is_visible(model: &NativeAppModel) -> bool {
    !matches!(model.update.status, NativeUpdateStatusModel::Idle)
        || model.update.available_version_label.is_some()
        || model.update.last_error.is_some()
}

/// Build update status and action automation targets.
fn update_panel_node(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let mut actions = Vec::new();
    if model.update.available_url.is_some() {
        actions.push("open_update_link");
    }
    if matches!(model.update.status, NativeUpdateStatusModel::Available) {
        actions.push("install_update");
    }
    if matches!(
        model.update.status,
        NativeUpdateStatusModel::Available | NativeUpdateStatusModel::Error
    ) {
        actions.push("dismiss_update");
    }
    let mut children = Vec::new();
    if actions.contains(&"open_update_link") {
        children.push(with_actions(
            automation_node(
                "shell.top_bar.update.open",
                NativeAutomationRole::Button,
                Some(String::from("Open update")),
                bounds(x, y, width / 3.0, height),
                None,
                false,
                Vec::new(),
            ),
            &["open_update_link"],
        ));
    }
    if actions.contains(&"install_update") {
        children.push(with_actions(
            automation_node(
                "shell.top_bar.update.install",
                NativeAutomationRole::Button,
                Some(String::from("Install update")),
                bounds(x + width / 3.0, y, width / 3.0, height),
                None,
                false,
                Vec::new(),
            ),
            &["install_update"],
        ));
    }
    if actions.contains(&"dismiss_update") {
        children.push(with_actions(
            automation_node(
                "shell.top_bar.update.dismiss",
                NativeAutomationRole::Button,
                Some(String::from("Dismiss update")),
                bounds(x + (width / 3.0) * 2.0, y, width / 3.0, height),
                None,
                false,
                Vec::new(),
            ),
            &["dismiss_update"],
        ));
    }
    let mut panel = with_owned_actions(
        automation_node(
            "shell.top_bar.update_panel",
            NativeAutomationRole::Panel,
            Some(model.update.status_label.clone()),
            bounds(x, y, width, height),
            model.update.available_version_label.clone(),
            false,
            children,
        ),
        actions,
    );
    panel.metadata.insert(
        String::from("status"),
        update_status_label(model.update.status).to_owned(),
    );
    panel
}

/// Format an optional normalized range for automation metadata.
fn normalized_range_micros(
    range: Option<&crate::app_core::actions::NativeNormalizedRangeModel>,
) -> String {
    range
        .map(|range| format!("{}-{}", range.start_micros, range.end_micros))
        .unwrap_or_default()
}

/// Convert update status into a stable automation metadata label.
fn update_status_label(status: NativeUpdateStatusModel) -> &'static str {
    match status {
        NativeUpdateStatusModel::Idle => "idle",
        NativeUpdateStatusModel::Checking => "checking",
        NativeUpdateStatusModel::Available => "available",
        NativeUpdateStatusModel::Error => "error",
    }
}

fn status_bar(
    width: f32,
    y: f32,
    height: f32,
    model: &NativeAppModel,
) -> NativeAutomationNodeSnapshot {
    let mut metadata = BTreeMap::new();
    metadata.insert(String::from("left"), model.status.left.clone());
    metadata.insert(String::from("center"), model.status.center.clone());
    metadata.insert(String::from("right"), model.status.right.clone());
    let mut node = automation_node(
        "shell.status_bar",
        NativeAutomationRole::Readout,
        Some(String::from("Status bar")),
        bounds(0.0, y, width, height),
        Some(model.status.center.clone()),
        false,
        Vec::new(),
    );
    node.metadata = metadata;
    node
}

fn automation_node(
    id: impl Into<String>,
    role: NativeAutomationRole,
    label: Option<String>,
    bounds: NativeAutomationBounds,
    value: Option<String>,
    selected: bool,
    children: Vec<NativeAutomationNodeSnapshot>,
) -> NativeAutomationNodeSnapshot {
    NativeAutomationNodeSnapshot {
        id: NativeAutomationNodeId::new(id),
        role,
        label,
        bounds,
        value,
        enabled: true,
        selected,
        available_actions: Vec::new(),
        metadata: BTreeMap::new(),
        children,
    }
}

/// Attach borrowed action IDs to a node.
fn with_actions(
    mut node: NativeAutomationNodeSnapshot,
    actions: &[&str],
) -> NativeAutomationNodeSnapshot {
    node.available_actions = actions.iter().map(|action| (*action).to_owned()).collect();
    node
}

/// Attach owned action IDs to a node.
fn with_owned_actions(
    mut node: NativeAutomationNodeSnapshot,
    actions: Vec<&str>,
) -> NativeAutomationNodeSnapshot {
    node.available_actions = actions.into_iter().map(str::to_owned).collect();
    node
}

fn bounds(x: f32, y: f32, width: f32, height: f32) -> NativeAutomationBounds {
    NativeAutomationBounds {
        x,
        y,
        width: width.max(0.0),
        height: height.max(0.0),
    }
}

/// Compute clamped centered overlay bounds for the current viewport.
fn centered_bounds(
    viewport_width: f32,
    viewport_height: f32,
    width: f32,
    height: f32,
) -> NativeAutomationBounds {
    bounds(
        ((viewport_width - width) * 0.5).max(0.0),
        ((viewport_height - height) * 0.5).max(0.0),
        width.min(viewport_width.max(0.0)),
        height.min(viewport_height.max(0.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_snapshot_adapter_exposes_shell_root_from_wavecrate_model() {
        let model = NativeAppModel::default();
        let snapshot = capture_gui_automation_snapshot([1440.0, 810.0], &model);

        assert_eq!(snapshot.root.id.0, "shell.root");
        assert_eq!(snapshot.viewport_width, 1440);
        assert_eq!(snapshot.viewport_height, 810);
    }
}
