use super::super::jobs::{
    JobMessage, SourceHydrationKind, SourceHydrationResult, SourceHydrationSnapshot,
};
use super::super::library::sources::hydration::with_source_hydration_async_enabled_for_tests;
use super::super::library::wavs::with_browser_async_pipeline_enabled_for_tests;
use super::super::test_support::sample_entry;
use super::super::*;
use super::common::visible_indices;
use crate::app::state::FolderPaneId;
use crate::sample_sources::Rating;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn build_controller_with_sources(names: &[&str]) -> (AppController, Vec<SampleSource>) {
    let mut controller = AppController::new(WaveformRenderer::new(16, 16), None);
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::mem::forget(dir);
    let mut sources = Vec::new();
    for name in names {
        let source_root = root.join(name);
        std::fs::create_dir_all(&source_root).unwrap();
        let source = SampleSource::new(source_root);
        controller.cache_db(&source).unwrap();
        controller.library.sources.push(source.clone());
        sources.push(source);
    }
    controller.assign_source_to_folder_pane(FolderPaneId::Upper, Some(sources[0].id.clone()));
    controller.selection_state.ctx.selected_source = Some(sources[0].id.clone());
    controller
        .selection_state
        .ctx
        .last_selected_browsable_source = Some(sources[0].id.clone());
    controller.refresh_sources_ui();
    (controller, sources)
}

fn cache_source_entries(
    controller: &mut AppController,
    source: &SampleSource,
    entries: Vec<WavEntry>,
) {
    let total = entries.len();
    controller.cache.wav.insert_page(
        source.id.clone(),
        total,
        controller.wav_entries.page_size,
        0,
        entries,
    );
}

fn upsert_source_db_entry(controller: &mut AppController, source: &SampleSource, entry: &WavEntry) {
    if let Some(parent) = entry.relative_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(source.root.join(parent)).unwrap();
    }
    let db = controller.database_for(source).unwrap();
    db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
        .unwrap();
    db.set_tag(&entry.relative_path, entry.tag).unwrap();
    db.set_looped(&entry.relative_path, entry.looped).unwrap();
    db.set_locked(&entry.relative_path, entry.locked).unwrap();
}

fn hydration_result(
    controller: &AppController,
    source: &SampleSource,
    request_id: u64,
    pane: FolderPaneId,
    kind: SourceHydrationKind,
    entries: Vec<WavEntry>,
    from_cache: bool,
) -> SourceHydrationResult {
    let available_folders = entries
        .iter()
        .filter_map(|entry| entry.relative_path.parent())
        .filter(|path| !path.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .collect::<BTreeSet<_>>();
    let folder_tree =
        crate::app::controller::library::source_folders::FolderTreeSnapshot::from_available(
            &available_folders,
        );
    let path_lookup = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            (
                PathBuf::from(entry.relative_path.to_string_lossy().replace('\\', "/")),
                index,
            )
        })
        .collect::<HashMap<_, _>>();
    SourceHydrationResult {
        request_id,
        pane,
        kind,
        source_id: source.id.clone(),
        elapsed: std::time::Duration::from_millis(5),
        result: Ok(SourceHydrationSnapshot {
            entries,
            total: path_lookup.len(),
            page_size: controller.wav_entries.page_size,
            path_lookup,
            available_folders,
            folder_tree,
            feature_cache: None,
            from_cache,
        }),
    }
}

#[test]
fn selecting_cached_source_clears_browser_until_async_hydration_applies() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("alpha.wav", Rating::NEUTRAL),
        sample_entry("beta.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_browser_lists();

    let cached_entries = vec![
        sample_entry("folder/kick.wav", Rating::KEEP_1),
        sample_entry("snare.wav", Rating::NEUTRAL),
    ];
    std::fs::create_dir_all(sources[1].root.join("folder")).unwrap();
    cache_source_entries(&mut controller, &sources[1], cached_entries.clone());

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index(1);

        assert_eq!(controller.ui.sources.selected, Some(1));
        assert_eq!(
            controller.ui.sources.loading_source_id,
            Some(sources[1].id.clone())
        );
        assert!(controller.ui.browser.search.source_loading);
        assert!(!controller.ui.browser.search.search_busy);
        assert!(visible_indices(&controller).is_empty());
        assert!(controller.ui.sources.folders.rows.is_empty());

        let request_id = controller
            .runtime
            .pending_active_source_hydration
            .as_ref()
            .expect("pending source hydration")
            .request_id;
        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[1],
                request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                cached_entries.clone(),
                true,
            ),
        ));
    });

    assert_eq!(visible_indices(&controller), vec![0, 1]);
    assert_eq!(
        controller.sample_view.wav.selected_wav,
        Some(PathBuf::from("folder/kick.wav"))
    );
    assert!(!controller.ui.browser.search.source_loading);
    assert!(
        !controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Upper)
            .loading
    );
}

#[test]
fn stale_uncached_source_hydration_result_is_dropped() {
    let (mut controller, sources) =
        build_controller_with_sources(&["source-a", "source-b", "source-c"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();

    let source_b_entries = vec![sample_entry("drums/kick.wav", Rating::NEUTRAL)];
    let source_c_entries = vec![sample_entry("vox.wav", Rating::KEEP_1)];
    std::fs::create_dir_all(sources[1].root.join("drums")).unwrap();
    for entry in &source_b_entries {
        upsert_source_db_entry(&mut controller, &sources[1], entry);
    }
    for entry in &source_c_entries {
        upsert_source_db_entry(&mut controller, &sources[2], entry);
    }

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index(1);
        let first_request_id = controller
            .runtime
            .pending_active_source_hydration
            .as_ref()
            .expect("first pending hydration")
            .request_id;

        controller.select_source_by_index(2);
        let second_request_id = controller
            .runtime
            .pending_active_source_hydration
            .as_ref()
            .expect("second pending hydration")
            .request_id;

        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[1],
                first_request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                source_b_entries.clone(),
                false,
            ),
        ));

        assert!(visible_indices(&controller).is_empty());
        assert_eq!(controller.selected_source_id(), Some(sources[2].id.clone()));
        assert!(controller.ui.browser.search.source_loading);

        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[2],
                second_request_id,
                FolderPaneId::Upper,
                SourceHydrationKind::ActiveSelection,
                source_c_entries.clone(),
                false,
            ),
        ));
    });

    assert_eq!(visible_indices(&controller), vec![0]);
    assert_eq!(
        controller.sample_view.wav.selected_wav,
        Some(PathBuf::from("vox.wav"))
    );
    assert!(!controller.ui.browser.search.source_loading);
}

#[test]
fn inactive_pane_source_hydration_keeps_active_browser_state_stable() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();
    controller.select_wav_by_index(0);

    let inactive_entries = vec![sample_entry("drums/kick.wav", Rating::NEUTRAL)];
    std::fs::create_dir_all(sources[1].root.join("drums")).unwrap();

    with_source_hydration_async_enabled_for_tests(true, || {
        controller.select_source_by_index_in_pane(FolderPaneId::Lower, 1);

        assert_eq!(controller.selected_source_id(), Some(sources[0].id.clone()));
        assert_eq!(visible_indices(&controller), vec![0]);
        assert_eq!(
            controller.sample_view.wav.selected_wav,
            Some(PathBuf::from("alpha.wav"))
        );
        assert!(
            controller
                .ui
                .sources
                .folder_pane(FolderPaneId::Lower)
                .loading
        );
        assert!(
            controller
                .ui
                .sources
                .folder_pane(FolderPaneId::Lower)
                .browser
                .rows
                .is_empty()
        );

        let request_id = controller
            .runtime
            .pending_inactive_source_hydration
            .as_ref()
            .expect("inactive pane hydration")
            .request_id;
        controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
            hydration_result(
                &controller,
                &sources[1],
                request_id,
                FolderPaneId::Lower,
                SourceHydrationKind::InactivePane,
                inactive_entries.clone(),
                true,
            ),
        ));
    });

    assert_eq!(controller.selected_source_id(), Some(sources[0].id.clone()));
    assert_eq!(
        controller.sample_view.wav.selected_wav,
        Some(PathBuf::from("alpha.wav"))
    );
    assert_eq!(visible_indices(&controller), vec![0]);
    assert!(
        !controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Lower)
            .loading
    );
    assert_eq!(
        controller
            .ui
            .sources
            .folder_pane(FolderPaneId::Lower)
            .browser
            .rows
            .len(),
        2
    );
}

#[test]
fn async_source_hydration_keeps_loading_until_async_browser_projection_applies() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller.set_wav_entries_for_tests(vec![sample_entry("alpha.wav", Rating::NEUTRAL)]);
    controller.rebuild_browser_lists();
    let hydrated_entries = vec![sample_entry("beta.wav", Rating::NEUTRAL)];

    with_source_hydration_async_enabled_for_tests(true, || {
        with_browser_async_pipeline_enabled_for_tests(true, || {
            controller.select_source_by_index(1);
            let request_id = controller
                .runtime
                .pending_active_source_hydration
                .as_ref()
                .expect("pending hydration")
                .request_id;
            controller.apply_background_job_message_for_tests(JobMessage::SourceHydrated(
                hydration_result(
                    &controller,
                    &sources[1],
                    request_id,
                    FolderPaneId::Upper,
                    SourceHydrationKind::ActiveSelection,
                    hydrated_entries.clone(),
                    true,
                ),
            ));

            assert!(controller.ui.browser.search.source_loading);
            assert!(controller.ui.browser.search.search_busy);

            let search_request_id = controller
                .runtime
                .pending_active_source_hydration
                .as_ref()
                .and_then(|pending| pending.search_request_id)
                .expect("queued browser search request");
            let visible = crate::app::state::VisibleRows::List(vec![0usize].into());
            controller.apply_background_job_message_for_tests(JobMessage::BrowserSearchFinished(
                crate::app::controller::jobs::SearchResult {
                    request_id: search_request_id,
                    source_id: sources[1].id.clone(),
                    query: String::new(),
                    visible,
                    trash: std::sync::Arc::from([]),
                    neutral: std::sync::Arc::from([0usize]),
                    keep: std::sync::Arc::from([]),
                    scores: std::sync::Arc::from([]),
                },
            ));
        });
    });

    assert!(!controller.ui.browser.search.source_loading);
    assert!(!controller.ui.browser.search.search_busy);
    assert_eq!(visible_indices(&controller), vec![0]);
}
