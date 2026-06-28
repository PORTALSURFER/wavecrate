pub(super) use super::super::stages::{BuildVisibleRowsParams, build_visible_rows_for_job};
pub(super) use super::super::*;
pub(super) use crate::app::controller::library::source_folders::FolderBrowserModel;
pub(super) use crate::app::controller::state::cache::FolderBrowserCacheKey;
pub(super) use crate::app::controller::test_support::prepare_with_source_and_wav_entries;
pub(super) use crate::app::state::FolderPaneId;
pub(super) use crate::sample_sources::{Rating, WavEntry};
pub(super) use std::collections::BTreeSet;
pub(super) use std::path::PathBuf;
pub(super) use std::sync::Arc;

pub(super) fn compact_entries(entries: &[WavEntry]) -> Vec<CompactSearchEntry> {
    entries
        .iter()
        .map(|entry| {
            let relative_path = entry.relative_path.to_string_lossy().to_string();
            let display_label = crate::app::view_model::sample_display_label(&entry.relative_path);
            CompactSearchEntry {
                display_label: display_label.into_boxed_str(),
                relative_path: relative_path.into(),
                tag: entry.tag,
                locked: entry.locked,
                last_played_at: entry.last_played_at,
                tag_named: false,
            }
        })
        .collect()
}

pub(super) fn search_entry(path: &str, tag: Rating) -> WavEntry {
    WavEntry {
        relative_path: PathBuf::from(path),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag,
        looped: false,
        sound_type: None,
        locked: false,
        missing: false,
        last_played_at: None,
        last_curated_at: None,
        user_tag: None,
        tag_named: false,
        normal_tags: Vec::new(),
    }
}

pub(super) fn visible_indices(controller: &crate::app::controller::AppController) -> Vec<usize> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.visible_browser_index(row))
        .collect()
}

pub(super) fn make_search_job(
    source: &crate::sample_sources::SampleSource,
    query: &str,
) -> SearchJob {
    SearchJob {
        request_id: 1,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        query: query.to_string(),
        filter: TriageFlagFilter::All,
        rating_filter: Default::default(),
        playback_age_filter: Default::default(),
        tag_named_filter: crate::app::state::TagNamedFilter::All,
        sidebar_filters: Default::default(),
        sidebar_bpm_values: Default::default(),
        sort: SampleBrowserSort::ListOrder,
        similar_query: None,
        duplicate_cleanup: None,
        folder_selection: None,
        folder_negated: None,
        file_scope_mode: crate::app::state::FolderFileScopeMode::AllDescendants,
        metadata_delta_paths: Vec::new(),
        playback_age_now_unix_secs: 0,
    }
}
