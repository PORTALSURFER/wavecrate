use super::*;
use crate::app::controller::jobs::{FolderScanResult, SearchResult};
use crate::app::controller::state::cache::FolderBrowserCacheKey;
use crate::app::controller::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry,
};
use crate::app::state::{FolderPaneId, TriageFlagColumn, VisibleRows};
use crate::sample_sources::Rating;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

#[test]
fn stale_folder_scan_message_keeps_pending_request_and_cached_state() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller
        .ui_cache
        .folders
        .models
        .entry(FolderBrowserCacheKey {
            pane: FolderPaneId::Upper,
            source_id: source.id.clone(),
        })
        .or_default()
        .disk_refresh_in_progress = true;
    let request_id = controller
        .runtime
        .jobs
        .request_folder_scan(source.id.clone(), source.root.clone());

    controller.handle_background_job_message(JobMessage::FolderScanFinished(FolderScanResult {
        request_id: request_id.wrapping_add(1),
        source_id: source.id.clone(),
        folders: BTreeSet::from([PathBuf::from("drums")]),
    }));

    assert_eq!(
        controller.runtime.jobs.pending_folder_scan_source(),
        Some(source.id.clone())
    );
    let model = controller
        .ui_cache
        .folders
        .models
        .get(&FolderBrowserCacheKey {
            pane: FolderPaneId::Upper,
            source_id: source.id.clone(),
        })
        .expect("folder model");
    assert!(model.disk_refresh_in_progress);
    assert!(model.disk_folders.is_empty());
}

#[test]
fn matching_browser_search_message_refreshes_visible_rows_and_clears_busy_state() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("kick.wav", Rating::NEUTRAL),
        sample_entry("snare.wav", Rating::TRASH_1),
        sample_entry("hat.wav", Rating::KEEP_1),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("hat.wav"));
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("snare.wav"));
    controller.ui.browser.search.search_query = "hat".into();
    controller.ui.browser.search.search_busy = true;
    controller.ui.browser.search.latest_search_request_id = 9;
    controller
        .ui
        .browser
        .search
        .latest_applied_search_request_id = 3;
    controller.ui.browser.viewport.visible_rows_revision = 14;
    controller.ui.browser.selection.marker_cache = Some(Default::default());
    controller.ui.browser.selection.selection_anchor_visible = Some(7);
    controller.ui.browser.selection.selected = None;
    controller.ui.browser.selection.loaded = None;
    controller.ui.browser.selection.selected_visible = None;
    controller.ui.browser.selection.loaded_visible = None;
    controller.set_ui_loaded_wav(None);

    controller.handle_background_job_message(JobMessage::BrowserSearchFinished(SearchResult {
        request_id: 9,
        source_id: source.id,
        query: "hat".into(),
        visible: VisibleRows::List(Arc::from([2usize, 0usize])),
        trash: Arc::from([1usize]),
        neutral: Arc::from([0usize]),
        keep: Arc::from([2usize]),
        scores: Arc::from([Some(11_i64), None, Some(42_i64)]),
    }));

    assert_eq!(controller.ui.browser.viewport.visible.len(), 2);
    assert_eq!(controller.ui.browser.viewport.visible_rows_revision, 15);
    assert_eq!(
        controller
            .ui
            .browser
            .search
            .latest_applied_search_request_id,
        9
    );
    assert!(!controller.ui.browser.search.search_busy);
    assert!(controller.ui.browser.selection.marker_cache.is_none());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(controller.ui.browser.selection.loaded_visible, None);
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    let selected = controller
        .ui
        .browser
        .selection
        .selected
        .expect("selected browser index");
    assert_eq!(selected.column, TriageFlagColumn::Keep);
    assert_eq!(selected.row, 0);
    let loaded = controller
        .ui
        .browser
        .selection
        .loaded
        .expect("loaded browser index");
    assert_eq!(loaded.column, TriageFlagColumn::Trash);
    assert_eq!(loaded.row, 0);
    assert_eq!(
        controller.ui.loaded_wav.as_deref(),
        Some(Path::new("snare.wav"))
    );
    let browser_search_revision = controller.ui.projection_revisions.browser_search;
    assert!(controller.refresh_projection_revision_bus());
    assert_ne!(
        controller.ui.projection_revisions.browser_search,
        browser_search_revision
    );
}

#[test]
fn stale_browser_search_message_leaves_visible_rows_and_busy_state_unchanged() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("kick.wav", Rating::NEUTRAL),
        sample_entry("snare.wav", Rating::NEUTRAL),
    ]);
    controller.ui.browser.search.search_query = "kick".into();
    controller.ui.browser.search.search_busy = true;
    controller.ui.browser.search.latest_search_request_id = 5;
    controller
        .ui
        .browser
        .search
        .latest_applied_search_request_id = 2;
    controller.ui.browser.viewport.visible_rows_revision = 8;
    let starting_visible_len = controller.ui.browser.viewport.visible.len();

    controller.handle_background_job_message(JobMessage::BrowserSearchFinished(SearchResult {
        request_id: 4,
        source_id: source.id,
        query: "kick".into(),
        visible: VisibleRows::List(Arc::from([0usize])),
        trash: Arc::from([]),
        neutral: Arc::from([0usize]),
        keep: Arc::from([]),
        scores: Arc::from([Some(7_i64), None]),
    }));

    assert_eq!(
        controller.ui.browser.viewport.visible.len(),
        starting_visible_len
    );
    assert_eq!(controller.ui.browser.viewport.visible_rows_revision, 8);
    assert_eq!(
        controller
            .ui
            .browser
            .search
            .latest_applied_search_request_id,
        2
    );
    assert!(controller.ui.browser.search.search_busy);
}
