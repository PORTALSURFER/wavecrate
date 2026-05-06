use super::*;
use crate::sample_sources::{WavEntry, db::SourceTagUsage};

/// Scalar inputs needed to project the retained browser row window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BrowserRowsProjectionInputs {
    /// Number of visible rows in the current browser list projection.
    pub visible_count: usize,
    /// Focused visible-row index, when any.
    pub selected_visible_row: Option<usize>,
    /// Visible-row anchor used by range selection, when any.
    pub anchor_visible_row: Option<usize>,
    /// Whether selection changes should auto-scroll the browser viewport.
    pub autoscroll: bool,
    /// Requested top visible-row index for manual browser viewport scrolling.
    pub view_start_row: usize,
}

/// Capture the current row-window projection inputs without rebuilding browser chrome.
pub(crate) fn project_browser_rows_projection_inputs(
    controller: &AppController,
) -> BrowserRowsProjectionInputs {
    BrowserRowsProjectionInputs {
        visible_count: controller.ui.browser.viewport.visible.len(),
        selected_visible_row: controller.ui.browser.selection.selected_visible,
        anchor_visible_row: controller.ui.browser.selection.selection_anchor_visible,
        autoscroll: controller.ui.browser.selection.autoscroll,
        view_start_row: controller.ui.browser.viewport.view_window_start,
    }
}

/// Project browser panel frame metadata without materializing row contents.
///
/// Callers can combine this with row-window projection helpers to refresh
/// metadata and row payloads independently when only one segment is dirty.
pub(crate) fn project_browser_panel_frame_model(
    controller: &mut AppController,
) -> BrowserPanelModel {
    let row_inputs = project_browser_rows_projection_inputs(controller);
    let selected_path_count = controller.ui.browser.selection.selected_paths.len();
    let search_query = controller.ui.browser.search.search_query.clone();
    let active_rating_filters =
        browser_rating_filter_flags(&controller.ui.browser.search.rating_filter);
    let active_playback_age_filters =
        browser_playback_age_filter_flags(&controller.ui.browser.search.playback_age_filter);
    let marked_filter_active = controller.ui.browser.search.marked_only;
    let tag_named_filter_active = !matches!(
        controller.ui.browser.search.tag_named_filter,
        crate::app_core::app_api::state::TagNamedFilter::All
    );
    let tag_named_filter_negated = matches!(
        controller.ui.browser.search.tag_named_filter,
        crate::app_core::app_api::state::TagNamedFilter::NotTagNamed
    );
    let search_placeholder = Some(super::browser_search_placeholder(
        controller.ui.browser.search.search_focus_requested,
    ));
    let busy = controller.ui.browser.search.search_busy;
    let duplicate_cleanup_active = controller.ui.browser.duplicate_cleanup.is_some();
    let similarity_filtered =
        !duplicate_cleanup_active && controller.ui.browser.search.similar_query.is_some();
    let sort_label = Some(if duplicate_cleanup_active {
        String::from("Duplicate cleanup")
    } else {
        super::browser_sort_label(SampleBrowserSort::from(controller.ui.browser.search.sort))
            .to_owned()
    });
    let active_tab_label =
        Some(super::browser_tab_label(controller.ui.browser.active_tab).to_owned());
    let focused_sample_label = project_browser_focused_sample_label(controller);
    let tag_sidebar = project_browser_tag_sidebar_model(controller);
    BrowserPanelModel {
        visible_count: row_inputs.visible_count,
        selected_visible_row: row_inputs.selected_visible_row,
        autoscroll: row_inputs.autoscroll,
        view_start_row: row_inputs.view_start_row,
        selected_path_count,
        search_query,
        active_rating_filters,
        active_playback_age_filters,
        marked_filter_active,
        tag_named_filter_active,
        tag_named_filter_negated,
        search_placeholder,
        busy,
        source_loading: controller.ui.browser.search.source_loading,
        metadata_pending: controller.selected_source_has_pending_metadata_mutations(),
        file_op_pending: controller.selected_source_has_pending_file_mutations()
            || controller.file_ops_in_progress_for_projection(),
        similarity_filtered,
        duplicate_cleanup_active,
        sort_label,
        active_tab_label,
        focused_sample_label,
        tag_sidebar,
        anchor_visible_row: row_inputs.anchor_visible_row,
        rows: RetainedVec::new(),
    }
}

/// Project browser panel metadata and row window into one panel model.
pub(crate) fn project_browser_model(controller: &mut AppController) -> BrowserPanelModel {
    let mut panel = project_browser_panel_frame_model(controller);
    panel.rows = super::project_browser_rows_model(
        controller,
        panel.visible_count,
        panel.selected_visible_row,
        panel.anchor_visible_row,
    );
    panel
}

/// Project active browser rating-filter levels into a fixed chip-state array.
fn browser_rating_filter_flags(rating_filter: &std::collections::BTreeSet<i8>) -> [bool; 8] {
    let mut flags = [false; 8];
    for (index, level) in [-3, -2, -1, 0, 1, 2, 3, 4].into_iter().enumerate() {
        flags[index] = rating_filter.contains(&level);
    }
    flags
}

/// Project active browser playback-age filters into a fixed chip-state array.
fn browser_playback_age_filter_flags(
    playback_age_filter: &std::collections::BTreeSet<PlaybackAgeFilterChip>,
) -> [bool; 3] {
    let mut flags = [false; 3];
    for (index, chip) in browser_playback_age_filter_chips().into_iter().enumerate() {
        flags[index] = playback_age_filter.contains(&chip);
    }
    flags
}

/// Project the browser's focused sample label from the selected sample path.
///
/// Browser metadata should stay aligned with the current browser selection when
/// the selection has already moved ahead of the async waveform-loading state.
/// Project the browser's focused sample label from the current target snapshot.
pub(crate) fn project_browser_focused_sample_label(controller: &AppController) -> Option<String> {
    controller
        .ui
        .browser
        .selection
        .last_focused_path
        .as_deref()
        .or_else(|| {
            controller
                .ui
                .browser
                .selection
                .selected_paths
                .first()
                .map(|path| path.as_path())
        })
        .or(controller.ui.loaded_wav.as_deref())
        .map(view_model::sample_display_label)
}

/// Project the browser tag sidebar from current target selection and metadata.
///
/// The retained projection cache keeps this surface on its own invalidation
/// contract because tag metadata edits and same-cardinality target swaps should
/// not depend on unrelated browser chrome churn.
pub(crate) fn project_browser_tag_sidebar_model(
    controller: &mut AppController,
) -> BrowserTagSidebarModel {
    let sidebar_targets = controller.browser_tag_sidebar_target_snapshot();
    let target_entries = sidebar_targets.resolve_entries(controller);
    let selected_count = target_entries.len();
    let header_label = match selected_count {
        0 => String::from("Select samples"),
        1 => target_entries[0]
            .relative_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| view_model::sample_display_label(&target_entries[0].relative_path)),
        count => format!("{count} samples selected"),
    };
    let exclusive_pills = [
        pill_model(
            "playback-loop",
            "Loop",
            bool_tag_state(&target_entries, |entry| entry.looped),
        ),
        pill_model(
            "playback-one-shot",
            "One-shot",
            bool_tag_state(&target_entries, |entry| !entry.looped),
        ),
    ];
    let (option_pills, create_pill) = if let Some(source) = controller.current_source() {
        let target_paths = sidebar_targets.resolve_paths(controller);
        project_normal_tag_candidates(controller, &source, &target_paths).unwrap_or_default()
    } else {
        (Vec::new(), None)
    };
    BrowserTagSidebarModel {
        // The tag editor is now rendered in the left library sidebar. Keep the
        // existing projection payload for mutation/input compatibility without
        // opening the old browser-row overlay.
        open: false,
        selected_count,
        header_label,
        primary_action_enabled: controller.ui.browser.tag_sidebar_auto_rename,
        input_value: controller.ui.browser.tag_sidebar_input.clone(),
        input_placeholder: String::from("Add tag"),
        exclusive_pills,
        option_pills,
        create_pill,
    }
}

fn pill_model(id: &str, label: &str, state: BrowserTagState) -> BrowserTagPillModel {
    BrowserTagPillModel {
        id: id.to_string(),
        label: label.to_string(),
        state,
    }
}

fn bool_tag_state(entries: &[WavEntry], predicate: impl Fn(&WavEntry) -> bool) -> BrowserTagState {
    if entries.is_empty() {
        return BrowserTagState::Off;
    }
    let on_count = entries.iter().filter(|entry| predicate(entry)).count();
    match on_count {
        0 => BrowserTagState::Off,
        count if count == entries.len() => BrowserTagState::On,
        _ => BrowserTagState::Mixed,
    }
}

fn project_normal_tag_candidates(
    controller: &mut AppController,
    source: &crate::sample_sources::SampleSource,
    paths: &[std::path::PathBuf],
) -> Result<(Vec<BrowserTagPillModel>, Option<BrowserTagPillModel>), String> {
    let input = controller.ui.browser.tag_sidebar_input.clone();
    let normalized_input = normalize_tag_input(&input);
    let usages = {
        let db = controller
            .database_for(source)
            .map_err(|err| err.to_string())?;
        if normalized_input.is_empty() {
            db.most_used_tags(18).map_err(|err| err.to_string())?
        } else {
            db.search_tags(&input, 18).map_err(|err| err.to_string())?
        }
    };
    let mut pills = Vec::new();
    for usage in usages.into_iter().filter(normal_tag_visible) {
        let state =
            controller.normal_tag_state_for_source(source, paths, &usage.tag.display_label)?;
        pills.push(pill_model(
            &usage.tag.display_label,
            &usage.tag.display_label,
            state,
        ));
    }
    let create_pill = if normalized_input.is_empty() || !pills.is_empty() {
        None
    } else {
        let display_label = display_tag_input(&input);
        Some(pill_model(
            &display_label,
            &format!("Create \"{display_label}\""),
            BrowserTagState::Off,
        ))
    };
    Ok((pills, create_pill))
}

fn normal_tag_visible(usage: &SourceTagUsage) -> bool {
    !matches!(
        usage.tag.normalized_text.as_str(),
        "loop" | "looped" | "one-shot" | "one shot" | "oneshot"
    )
}

fn normalize_tag_input(input: &str) -> String {
    display_tag_input(input).to_ascii_lowercase()
}

fn display_tag_input(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}
