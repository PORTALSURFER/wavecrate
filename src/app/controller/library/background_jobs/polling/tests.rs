use super::*;
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage,
    FileOpResult, FolderScanResult, FocusedSimilarityPaths, FocusedSimilarityResult,
    LoadedSimilarityQueryResult, RetainedDeleteResolutionMode, RetainedDeleteResolutionResult,
    SearchResult, SelectionExportMessage,
};
use crate::app::controller::playback::audio_loader::{AudioLoadOutcome, AudioTransientResult};
use crate::app::controller::state::audio::LoadedAudio;
use crate::app::controller::state::audio::PendingAudio;
use crate::app::controller::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::state::{ProgressTaskKind, SampleBrowserSort, SimilarQuery, TriageFlagColumn, VisibleRows};
use crate::sample_sources::Rating;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool, mpsc::channel};

fn decode_audio_outcome(
    controller: &AppController,
    source: &SampleSource,
    relative_path: &Path,
) -> AudioLoadOutcome {
    let metadata = controller
        .current_file_metadata(source, relative_path)
        .expect("metadata");
    let bytes: Arc<[u8]> = controller
        .read_waveform_bytes(source, relative_path)
        .expect("waveform bytes")
        .into();
    let decoded = Arc::new(
        controller
            .sample_view
            .renderer
            .decode_from_bytes(bytes.as_ref())
            .expect("decoded waveform"),
    );
    AudioLoadOutcome {
        decoded,
        bytes,
        metadata,
        transients: None,
        stretched: false,
    }
}

#[test]
fn stale_folder_scan_message_keeps_pending_request_and_cached_state() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller
        .ui_cache
        .folders
        .models
        .entry(source.id.clone())
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
        .get(&source.id)
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

#[test]
fn file_ops_messages_update_progress_and_clear_active_overlay_on_finish() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(ProgressTaskKind::FileOps, "Copying files", 5, true);
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Progress {
        completed: 2,
        detail: Some("Copying kick.wav".into()),
    }));

    assert_eq!(controller.ui.progress.completed, 2);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Copying kick.wav")
    );
    assert!(controller.runtime.jobs.file_ops_in_progress());

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(
        FileOpResult::ClipboardPaste(ClipboardPasteResult {
            outcome: ClipboardPasteOutcome::Source {
                source_id: source.id,
                added: Vec::new(),
            },
            skipped: 0,
            errors: Vec::new(),
            cancelled: true,
            target_label: "Source".into(),
            action_past_tense: "Pasted",
        }),
    )));

    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
    assert_eq!(controller.ui.progress.task, None);
}

#[test]
fn retained_delete_resolution_result_clears_busy_scope_and_progress() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.show_status_progress(
        ProgressTaskKind::FileOps,
        "Restoring retained deletes",
        1,
        false,
    );
    controller.runtime.active_retained_delete_resolution = Some(ActiveRetainedDeleteResolution {
        entries: Vec::new(),
    });
    let (tx, rx) = channel();
    controller
        .runtime
        .jobs
        .start_file_ops(rx, Arc::new(AtomicBool::new(false)));
    drop(tx);

    controller.handle_background_job_message(JobMessage::FileOps(FileOpMessage::Finished(
        FileOpResult::RetainedDeleteResolution(RetainedDeleteResolutionResult {
            mode: RetainedDeleteResolutionMode::Restore,
            resolved: 1,
            affected_sources: vec![source.id],
            scan_sources: Vec::new(),
            failures: Vec::new(),
            recovery_report: crate::app::controller::library::source_folders::delete_recovery::DeleteRecoveryReport {
                entries: Vec::new(),
                retained_entries: Vec::new(),
                errors: Vec::new(),
            },
        }),
    )));

    assert!(
        controller
            .runtime
            .active_retained_delete_resolution
            .is_none()
    );
    assert!(!controller.runtime.jobs.file_ops_in_progress());
    assert!(!controller.ui.progress.visible);
}

#[test]
fn selection_export_progress_message_updates_status_bar_progress() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.jobs.set_pending_slice_batch_export(Some(
        crate::app::controller::jobs::PendingSliceBatchExport {
            request_id: 23,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("clip.wav"),
        },
    ));

    controller.handle_background_job_message(JobMessage::SelectionExport(
        SelectionExportMessage::Progress {
            request_id: 23,
            total: 4,
            completed: 2,
            detail: Some("Saving clip_slice002.wav".into()),
        },
    ));

    assert!(controller.ui.progress.visible);
    assert!(!controller.ui.progress.modal);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::SelectionExport)
    );
    assert_eq!(controller.ui.progress.total, 4);
    assert_eq!(controller.ui.progress.completed, 2);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("Saving clip_slice002.wav")
    );
}

#[test]
/// Primary audio completions should ignore stale requests and keep loading active until visuals arrive.
fn audio_primary_message_ignores_stale_completion_then_applies_matching_result() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("match.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("match.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller.ui.waveform.loading = Some(relative_path.to_path_buf());
    controller
        .runtime
        .jobs
        .set_pending_audio(Some(PendingAudio {
            request_id: 17,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            intent: AudioLoadIntent::Selection,
        }));

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Primary {
            request_id: 18,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            result: Ok(decode_audio_outcome(&controller, &source, relative_path)),
        },
    ));

    let pending = controller.runtime.jobs.pending_audio();
    assert!(pending.is_some(), "stale completion should stay pending");
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(relative_path)
    );
    assert!(controller.sample_view.wav.loaded_audio.is_none());

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Primary {
            request_id: 17,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            result: Ok(decode_audio_outcome(&controller, &source, relative_path)),
        },
    ));

    assert!(controller.runtime.jobs.pending_audio().is_none());
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(relative_path)
    );
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(relative_path)
    );
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.source_id),
        Some(&source.id)
    );
}

#[test]
/// Transient completions should route through the controller and refresh the active waveform UI.
fn audio_transients_message_routes_to_loaded_waveform_state() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("route.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("route.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller
        .load_waveform_for_selection(&source, relative_path)
        .expect("initial waveform load");
    let metadata = controller
        .current_file_metadata(&source, relative_path)
        .expect("metadata");
    let cache_token = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .cache_token;
    controller.ui.waveform.transients = Arc::from([]);
    controller.ui.waveform.transient_cache_token = None;

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Transients(AudioTransientResult {
            request_id: 17,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            metadata,
            cache_token,
            transients: Arc::from(vec![0.2, 0.7]),
            stretched: true,
        }),
    ));

    assert_eq!(controller.ui.waveform.transients.as_ref(), &[0.2, 0.7]);
    assert_eq!(
        controller.ui.waveform.transient_cache_token,
        Some(cache_token)
    );
}

#[test]
fn focused_similarity_message_ignores_stale_result_then_applies_matching_highlight() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.runtime.pending_focused_similarity_query = Some(
        crate::app::controller::state::runtime::PendingFocusedSimilarityQuery {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
        },
    );

    controller.apply_background_job_message_for_tests(JobMessage::FocusedSimilarityLoaded(
        FocusedSimilarityResult {
            request_id: 8,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(Some(FocusedSimilarityPaths {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                paths: vec![PathBuf::from("two.wav")],
                scores: vec![0.98],
                anchor_index: Some(0),
            })),
        },
    ));

    assert!(controller.runtime.pending_focused_similarity_query.is_some());
    assert!(controller.ui.browser.search.focused_similarity.is_none());

    controller.apply_background_job_message_for_tests(JobMessage::FocusedSimilarityLoaded(
        FocusedSimilarityResult {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(Some(FocusedSimilarityPaths {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                paths: vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")],
                scores: vec![0.99, 0.98],
                anchor_index: Some(0),
            })),
        },
    ));

    assert!(controller.runtime.pending_focused_similarity_query.is_none());
    let highlight = controller
        .ui
        .browser
        .search
        .focused_similarity
        .as_ref()
        .expect("focused similarity");
    assert_eq!(highlight.indices, vec![1]);
    assert_eq!(highlight.scores, vec![0.98]);
    assert_eq!(highlight.anchor_index, Some(0));
}

#[test]
fn loaded_similarity_query_message_ignores_stale_result_then_applies_matching_query() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similarity_sort_follow_loaded = true;
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        bytes: Arc::from(Vec::<u8>::new()),
        duration_seconds: 1.0,
        sample_rate: 44_100,
    });
    controller.runtime.pending_loaded_similarity_query = Some(
        crate::app::controller::state::runtime::PendingLoadedSimilarityQuery {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
        },
    );

    controller.apply_background_job_message_for_tests(JobMessage::LoadedSimilarityQueryBuilt(
        LoadedSimilarityQueryResult {
            request_id: 8,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(SimilarQuery {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                label: "Loaded: one.wav".to_string(),
                indices: vec![0, 1],
                scores: vec![1.0, 0.8],
                anchor_index: Some(0),
            }),
        },
    ));

    assert!(controller.runtime.pending_loaded_similarity_query.is_some());
    assert!(controller.ui.browser.search.similar_query.is_none());

    controller.apply_background_job_message_for_tests(JobMessage::LoadedSimilarityQueryBuilt(
        LoadedSimilarityQueryResult {
            request_id: 7,
            source_id: source.id.clone(),
            relative_path: PathBuf::from("one.wav"),
            result: Ok(SimilarQuery {
                sample_id: format!("{}::one.wav", source.id.as_str()),
                label: "Loaded: one.wav".to_string(),
                indices: vec![0, 1],
                scores: vec![1.0, 0.8],
                anchor_index: Some(0),
            }),
        },
    ));

    assert!(controller.runtime.pending_loaded_similarity_query.is_none());
    let query = controller
        .ui
        .browser
        .search
        .similar_query
        .as_ref()
        .expect("similarity query");
    assert_eq!(query.indices, vec![0, 1]);
    assert_eq!(query.anchor_index, Some(0));
}

#[test]
fn poll_background_jobs_limits_messages_per_pass() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    let sender = controller.runtime.jobs.message_sender();
    for _ in 0..(MAX_BACKGROUND_MESSAGES_PER_POLL + 2) {
        sender
            .send(JobMessage::Analysis(AnalysisJobMessage::Progress {
                source_id: Some(source.id.clone()),
                progress: crate::app::controller::library::analysis_jobs::AnalysisProgress {
                    pending: 2,
                    running: 1,
                    done: 3,
                    failed: 0,
                    samples_total: 5,
                    samples_pending_or_running: 2,
                },
            }))
            .expect("queue analysis progress");
    }

    controller.poll_background_jobs();

    let mut remaining = 0usize;
    while controller.runtime.jobs.try_recv_message().is_ok() {
        remaining += 1;
    }
    assert_eq!(remaining, 2);
}
