use super::*;
use crate::sample_sources::{SampleSoundType, WavEntry};

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
        rows: radiant::app::RetainedVec::new(),
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
fn project_browser_focused_sample_label(controller: &AppController) -> Option<String> {
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

fn project_browser_tag_sidebar_model(
    controller: &mut AppController,
) -> radiant::app::BrowserTagSidebarModel {
    let is_list_tab = matches!(controller.ui.browser.active_tab, SampleBrowserTab::List);
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
    let playback_type_pills = [
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
    let sound_type_pills = canonical_sound_type_pills(&target_entries);
    let custom_tag_state = string_tag_summary(&target_entries, |entry| entry.user_tag.as_deref());
    radiant::app::BrowserTagSidebarModel {
        open: is_list_tab && controller.ui.browser.tag_sidebar_open,
        selected_count,
        header_label,
        input_value: controller.ui.browser.tag_sidebar_input.clone(),
        input_placeholder: String::from("Add tag"),
        playback_type_pills,
        sound_type_pills,
        custom_tag_pill: custom_tag_state
            .1
            .map(|label| pill_model("custom-tag", label.as_str(), custom_tag_state.0)),
    }
}

fn pill_model(
    id: &str,
    label: &str,
    state: radiant::app::BrowserTagState,
) -> radiant::app::BrowserTagPillModel {
    radiant::app::BrowserTagPillModel {
        id: id.to_string(),
        label: label.to_string(),
        state,
    }
}

fn bool_tag_state(
    entries: &[WavEntry],
    predicate: impl Fn(&WavEntry) -> bool,
) -> radiant::app::BrowserTagState {
    if entries.is_empty() {
        return radiant::app::BrowserTagState::Off;
    }
    let on_count = entries.iter().filter(|entry| predicate(entry)).count();
    match on_count {
        0 => radiant::app::BrowserTagState::Off,
        count if count == entries.len() => radiant::app::BrowserTagState::On,
        _ => radiant::app::BrowserTagState::Mixed,
    }
}

fn canonical_sound_type_pills(entries: &[WavEntry]) -> Vec<radiant::app::BrowserTagPillModel> {
    [
        (SampleSoundType::Kick, "Kick"),
        (SampleSoundType::Snare, "Snare"),
        (SampleSoundType::Clap, "Clap"),
        (SampleSoundType::Hat, "Hi-hat"),
        (SampleSoundType::Perc, "Perc"),
        (SampleSoundType::Tom, "Tom"),
        (SampleSoundType::Rim, "Rim"),
        (SampleSoundType::Bass, "Bass"),
        (SampleSoundType::Sub, "Sub"),
        (SampleSoundType::Chord, "Chord"),
        (SampleSoundType::Stab, "Stab"),
        (SampleSoundType::Pad, "Pad"),
        (SampleSoundType::Lead, "Lead"),
        (SampleSoundType::Arp, "Arp"),
        (SampleSoundType::Seq, "Seq"),
        (SampleSoundType::Vocal, "Vocal"),
        (SampleSoundType::Fx, "FX"),
        (SampleSoundType::Texture, "Texture"),
    ]
    .into_iter()
    .map(|(sound_type, label)| {
        pill_model(
            sound_type.token(),
            label,
            option_tag_state(entries, |entry| entry.sound_type, sound_type),
        )
    })
    .collect()
}

fn option_tag_state<T: Copy + PartialEq>(
    entries: &[WavEntry],
    value_of: impl Fn(&WavEntry) -> Option<T>,
    expected: T,
) -> radiant::app::BrowserTagState {
    if entries.is_empty() {
        return radiant::app::BrowserTagState::Off;
    }
    let on_count = entries
        .iter()
        .filter(|entry| value_of(entry) == Some(expected))
        .count();
    match on_count {
        0 => radiant::app::BrowserTagState::Off,
        count if count == entries.len() => radiant::app::BrowserTagState::On,
        _ => radiant::app::BrowserTagState::Mixed,
    }
}

fn string_tag_summary<'a>(
    entries: &'a [WavEntry],
    value_of: impl Fn(&'a WavEntry) -> Option<&'a str>,
) -> (radiant::app::BrowserTagState, Option<String>) {
    if entries.is_empty() {
        return (radiant::app::BrowserTagState::Off, None);
    }
    let mut values = entries
        .iter()
        .filter_map(|entry| value_of(entry))
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    match values.len() {
        0 => (radiant::app::BrowserTagState::Off, None),
        1 => {
            let on_count = entries
                .iter()
                .filter(|entry| value_of(entry) == values.first().copied())
                .count();
            let state = if on_count == entries.len() {
                radiant::app::BrowserTagState::On
            } else {
                radiant::app::BrowserTagState::Mixed
            };
            (state, values.first().map(|value| (*value).to_string()))
        }
        _ => (
            radiant::app::BrowserTagState::Mixed,
            Some(String::from("Custom")),
        ),
    }
}
