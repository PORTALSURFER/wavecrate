use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::analysis_jobs::{self, AnalysisJobMessage};
use crate::app::controller::state::cache::{FeatureCache, FeatureCacheKey};
use crate::app::controller::test_support::{dummy_controller, write_test_wav};
use crate::app::controller::{SimilarityPrepStage, SimilarityPrepState};
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::scanner::{
    ChangedSample, RenamedSample, ScanError, ScanStats, UpdatedSample,
};
use crate::sample_sources::{Rating, ScanMode, SourceId};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

fn wait_for_analysis_message<F>(controller: &mut AppController, predicate: F) -> AnalysisJobMessage
where
    F: Fn(&AnalysisJobMessage) -> bool,
{
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message)) if predicate(&message) => return message,
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("expected analysis message, got {err:?}"),
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for analysis message"
        );
    }
}

fn assert_no_analysis_message(controller: &mut AppController) {
    let deadline = Instant::now() + Duration::from_millis(150);
    loop {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message)) => {
                panic!("unexpected analysis message: {message:?}");
            }
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) if Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(err) => panic!("unexpected receive error: {err:?}"),
        }
    }
}

fn assert_no_analysis_jobs_inserted(source: &SampleSource) {
    let conn = crate::sample_sources::SourceDatabase::open_connection(&source.root)
        .expect("open source db");
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM analysis_jobs
             WHERE job_type IN (?1, ?2)",
            rusqlite::params![
                analysis_jobs::db::ANALYZE_SAMPLE_JOB_TYPE,
                analysis_jobs::db::EMBEDDING_BACKFILL_JOB_TYPE,
            ],
            |row| row.get(0),
        )
        .expect("count analysis jobs");
    assert_eq!(count, 0, "scan completion inserted analysis jobs");
}

fn scan_result(
    source_id: SourceId,
    mode: ScanMode,
    kind: ScanKind,
    result: Result<ScanStats, ScanError>,
) -> ScanResult {
    ScanResult {
        source_id,
        mode,
        kind,
        result,
    }
}

fn changed_sample(relative_path: &str) -> ChangedSample {
    ChangedSample {
        relative_path: PathBuf::from(relative_path),
        file_size: 8,
        modified_ns: 42,
        content_hash: "hash-v2".to_string(),
    }
}

fn updated_sample(relative_path: &str) -> UpdatedSample {
    UpdatedSample {
        relative_path: PathBuf::from(relative_path),
        file_size: 8,
        modified_ns: 42,
        content_hash: None,
    }
}

fn renamed_sample(old_relative_path: &str, new_relative_path: &str) -> RenamedSample {
    RenamedSample {
        old_relative_path: PathBuf::from(old_relative_path),
        new_relative_path: PathBuf::from(new_relative_path),
        file_size: 8,
        modified_ns: 42,
        content_hash: None,
    }
}

#[test]
fn changed_scan_refreshes_selected_source_without_enqueuing_follow_up_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("kick.wav");
    write_test_wav(&wav_path, &[0.1, -0.1, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("kick.wav"))
        .expect("sample db entry");
    controller.ui_cache.browser.features.insert(
        source.id.clone(),
        FeatureCache {
            key: FeatureCacheKey::default(),
            rows: Vec::new().into(),
        },
    );
    controller.ui_cache.browser.durations.insert(
        source.id.clone(),
        HashMap::from([(PathBuf::from("kick.wav"), 1.25)]),
    );
    controller.show_status_progress(ProgressTaskKind::Scan, "Scanning source", 0, true);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Manual,
            Ok(ScanStats {
                added: 1,
                updated: 0,
                missing: 0,
                changed_samples: vec![changed_sample("kick.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
    assert!(
        !controller
            .ui_cache
            .browser
            .features
            .contains_key(&source.id)
    );
    assert!(controller.wav_entries.source_id.as_ref() == Some(&source.id));
    assert_eq!(controller.wav_entries.total, 1);
    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn rename_only_quick_scan_applies_anchored_browser_delta_without_wav_reload() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    controller.set_wav_entries_for_tests(vec![
        crate::app::controller::test_support::sample_entry("alpha.wav", Rating::NEUTRAL),
        crate::app::controller::test_support::sample_entry("old.wav", Rating::KEEP_1),
        crate::app::controller::test_support::sample_entry("zulu.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.select_wav_by_index(1);
    controller.ui.browser.selection.selection_anchor_visible = Some(1);
    controller.ui.browser.viewport.view_window_start = 1;
    let visible_revision = controller.ui.browser.viewport.visible_rows_revision;

    let db = controller.database_for(&source).expect("source db");
    db.upsert_file(Path::new("renamed.wav"), 8, 42)
        .expect("upsert renamed row");
    db.set_tag(Path::new("renamed.wav"), Rating::KEEP_1)
        .expect("set tag");
    drop(db);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats {
                updated: 1,
                renames_reconciled: 1,
                renamed_samples: vec![renamed_sample("old.wav", "renamed.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    assert_eq!(controller.wav_entries.total, 3);
    assert!(
        controller
            .wav_index_for_path(Path::new("old.wav"))
            .is_none()
    );
    assert_eq!(
        controller.wav_index_for_path(Path::new("renamed.wav")),
        Some(1)
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("renamed.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(controller.ui.browser.viewport.view_window_start, 1);
    assert_eq!(
        controller.ui.browser.viewport.visible_rows_revision,
        visible_revision
    );
    assert_no_analysis_message(&mut controller);
}

#[test]
fn small_updated_quick_scan_patches_cached_entry_without_wav_reload() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    controller.set_wav_entries_for_tests(vec![crate::app::controller::test_support::sample_entry(
        "kick.wav",
        Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    let db = controller.database_for(&source).expect("source db");
    db.upsert_file(Path::new("kick.wav"), 8, 42)
        .expect("upsert updated row");
    drop(db);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats {
                updated: 1,
                updated_samples: vec![updated_sample("kick.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    let index = controller
        .wav_index_for_path(Path::new("kick.wav"))
        .expect("updated row remains loaded");
    let entry = controller.wav_entries.entry(index).expect("entry");
    assert_eq!(entry.file_size, 8);
    assert_eq!(entry.modified_ns, 42);
    assert_eq!(controller.wav_entries.total, 1);
    assert_no_analysis_message(&mut controller);
}

#[test]
fn unchanged_scan_stays_analysis_free_when_similarity_prep_is_idle() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("snare.wav");
    write_test_wav(&wav_path, &[0.3, -0.3, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("snare.wav"))
        .expect("sample db entry");

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Hard,
            ScanKind::Manual,
            Ok(ScanStats::default()),
        ),
    );

    assert!(controller.wav_entries.source_id.as_ref() == Some(&source.id));
    assert_eq!(controller.wav_entries.total, 1);
    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn auto_changed_scan_refreshes_selected_source_without_enqueueing_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("kick.wav");
    write_test_wav(&wav_path, &[0.1, -0.1, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("kick.wav"))
        .expect("sample db entry");

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats {
                added: 1,
                changed_samples: vec![changed_sample("kick.wav")],
                ..ScanStats::default()
            }),
        ),
    );

    assert_eq!(controller.ui.progress.task, None);
    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn auto_unchanged_scan_does_not_backfill_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("snare.wav");
    write_test_wav(&wav_path, &[0.3, -0.3, 0.2, -0.2]);
    controller
        .ensure_sample_db_entry(&source, Path::new("snare.wav"))
        .expect("sample db entry");

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Auto,
            Ok(ScanStats::default()),
        ),
    );

    assert_no_analysis_message(&mut controller);
    assert_no_analysis_jobs_inserted(&source);
}

#[test]
fn unchanged_scan_finishes_similarity_prep_with_explicit_bootstrap_enqueue() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity.prep = Some(SimilarityPrepState {
        source_id: source.id.clone(),
        stage: SimilarityPrepStage::AwaitScan,
        umap_version: "v1".to_string(),
        scan_completed_at: None,
        skip_backfill: false,
        force_full_analysis: false,
    });
    controller.show_status_progress(ProgressTaskKind::Analysis, "Preparing similarity", 1, true);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Manual,
            Ok(ScanStats::default()),
        ),
    );

    let prep = controller
        .runtime
        .similarity
        .prep
        .as_ref()
        .expect("similarity prep");
    assert!(matches!(
        prep.stage,
        SimilarityPrepStage::AwaitEmbeddings | SimilarityPrepStage::Finalizing
    ));
    assert!(!prep.skip_backfill);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::Analysis)
    );
    assert!(controller.ui.progress.visible);
    match wait_for_analysis_message(&mut controller, |message| {
        matches!(message, AnalysisJobMessage::EnqueueFinished { .. })
    }) {
        AnalysisJobMessage::EnqueueFinished { announce: true, .. } => {}
        other => panic!("unexpected analysis message: {other:?}"),
    }
}

#[test]
fn canceled_scan_clears_similarity_prep_and_reports_warning_for_selected_source() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity.prep = Some(SimilarityPrepState {
        source_id: source.id.clone(),
        stage: SimilarityPrepStage::AwaitScan,
        umap_version: "v1".to_string(),
        scan_completed_at: None,
        skip_backfill: false,
        force_full_analysis: false,
    });
    controller.show_status_progress(ProgressTaskKind::Analysis, "Preparing similarity", 1, true);

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Quick,
            ScanKind::Manual,
            Err(ScanError::Canceled),
        ),
    );

    assert_eq!(controller.ui.status.text, "Quick sync canceled");
    assert!(controller.runtime.similarity.prep.is_none());
    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
}

#[test]
fn failed_scan_clears_similarity_prep_and_reports_error_for_selected_source() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity.prep = Some(SimilarityPrepState {
        source_id: source.id.clone(),
        stage: SimilarityPrepStage::AwaitScan,
        umap_version: "v1".to_string(),
        scan_completed_at: None,
        skip_backfill: false,
        force_full_analysis: false,
    });

    handle_scan_finished(
        &mut controller,
        scan_result(
            source.id.clone(),
            ScanMode::Hard,
            ScanKind::Manual,
            Err(ScanError::InvalidRoot(PathBuf::from("missing"))),
        ),
    );

    assert!(controller.ui.status.text.starts_with("Hard sync failed: "));
    assert!(controller.runtime.similarity.prep.is_none());
}

#[test]
fn scan_progress_updates_keep_indeterminate_total_and_path_detail() {
    let (mut controller, _source) = dummy_controller();
    controller.show_status_progress(ProgressTaskKind::Scan, "Scanning source", 0, true);

    handle_scan_progress(&mut controller, 12, Some(String::from("drums\\kick.wav")));

    assert_eq!(controller.ui.progress.task, Some(ProgressTaskKind::Scan));
    assert_eq!(controller.ui.progress.total, 0);
    assert_eq!(controller.ui.progress.completed, 12);
    assert_eq!(
        controller.ui.progress.detail.as_deref(),
        Some("drums\\kick.wav")
    );
}
