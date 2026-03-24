use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::analysis_jobs::AnalysisJobMessage;
use crate::app::controller::state::cache::FeatureCache;
use crate::app::controller::test_support::{dummy_controller, write_test_wav};
use crate::app::controller::{SimilarityPrepStage, SimilarityPrepState};
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::scanner::{ChangedSample, ScanError, ScanStats};
use crate::sample_sources::{ScanMode, SourceId};
use std::collections::HashMap;
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

#[test]
fn changed_scan_refreshes_selected_source_and_enqueues_follow_up_analysis() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("kick.wav");
    write_test_wav(&wav_path, &[0.1, -0.1, 0.2, -0.2]);
    controller
        .ui_cache
        .browser
        .features
        .insert(source.id.clone(), FeatureCache { rows: Vec::new() });
    controller.ui_cache.browser.durations.insert(
        source.id.clone(),
        HashMap::from([(PathBuf::from("kick.wav"), 1.25)]),
    );
    controller.show_status_progress(ProgressTaskKind::Scan, "Scanning", 1, true);

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

    match wait_for_analysis_message(&mut controller, |message| {
        matches!(message, AnalysisJobMessage::EnqueueFinished { .. })
    }) {
        AnalysisJobMessage::EnqueueFinished {
            inserted: _,
            progress: _,
            announce: true,
        } => {}
        other => panic!("unexpected analysis message: {other:?}"),
    }
}

#[test]
fn unchanged_scan_backfills_when_similarity_prep_is_idle() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).expect("cache db");
    let wav_path = source.root.join("snare.wav");
    write_test_wav(&wav_path, &[0.3, -0.3, 0.2, -0.2]);

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
    match wait_for_analysis_message(&mut controller, |message| {
        matches!(message, AnalysisJobMessage::EnqueueFinished { .. })
    }) {
        AnalysisJobMessage::EnqueueFinished { .. } => {}
        other => panic!("unexpected analysis message: {other:?}"),
    }
}

#[test]
fn unchanged_scan_finishes_similarity_prep_without_backfill_enqueue() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity_prep = Some(SimilarityPrepState {
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
        .similarity_prep
        .as_ref()
        .expect("similarity prep");
    assert!(matches!(
        prep.stage,
        SimilarityPrepStage::AwaitEmbeddings | SimilarityPrepStage::Finalizing
    ));
    assert!(prep.skip_backfill);
}

#[test]
fn canceled_scan_clears_similarity_prep_and_reports_warning_for_selected_source() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity_prep = Some(SimilarityPrepState {
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
    assert!(controller.runtime.similarity_prep.is_none());
    assert_eq!(controller.ui.progress.task, None);
    assert!(!controller.ui.progress.visible);
}

#[test]
fn failed_scan_clears_similarity_prep_and_reports_error_for_selected_source() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.runtime.similarity_prep = Some(SimilarityPrepState {
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
    assert!(controller.runtime.similarity_prep.is_none());
}
