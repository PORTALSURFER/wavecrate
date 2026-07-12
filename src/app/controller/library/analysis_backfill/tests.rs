use super::*;
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::test_support::{dummy_controller, sample_entry, write_test_wav};
use std::time::{Duration, Instant};

fn wait_for_analysis_message(
    controller: &mut AppController,
    mut predicate: impl FnMut(&analysis_jobs::AnalysisJobMessage) -> bool,
) -> analysis_jobs::AnalysisJobMessage {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match controller.runtime.jobs.try_recv_message() {
            Ok(JobMessage::Analysis(message)) if predicate(&message) => return message,
            Ok(_) => {}
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("unexpected receive error: {err:?}"),
        }
    }
    panic!("timed out waiting for analysis message");
}

fn pending_job_count(source: &SampleSource, job_type: &str) -> i64 {
    analysis_jobs::db::open_source_db(&source.root)
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs WHERE job_type = ?1 AND status = 'pending'",
            rusqlite::params![job_type],
            |row| row.get(0),
        )
        .unwrap()
}

fn prepare_manual_reanalysis_fixture(
    entries: &[&str],
) -> (
    AppController,
    SampleSource,
    tempfile::TempDir,
    crate::app_dirs::ConfigBaseGuard,
) {
    let config_dir = tempfile::tempdir().unwrap();
    let guard = crate::app_dirs::ConfigBaseGuard::set(config_dir.path().to_path_buf());
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let wav_entries: Vec<_> = entries
        .iter()
        .map(|entry| {
            let path = source.root.join(entry);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            write_test_wav(&path, &[0.0, 1.0, 0.0, -1.0]);
            sample_entry(entry, crate::sample_sources::Rating::NEUTRAL)
        })
        .collect();
    controller.set_wav_entries_for_tests(wav_entries);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    (controller, source, config_dir, guard)
}

#[test]
fn trigger_policy_matrix_keeps_implicit_reasons_disabled() {
    let cases = [
        (
            AnalysisTriggerReason::SampleAdded,
            AnalysisTriggerPolicy::ChangedSamples,
        ),
        (
            AnalysisTriggerReason::AudioContentChanged,
            AnalysisTriggerPolicy::ChangedSamples,
        ),
        (
            AnalysisTriggerReason::UserRequestedReanalysis,
            AnalysisTriggerPolicy::UserRequestedReanalysis,
        ),
        (
            AnalysisTriggerReason::SimilarityPrepBootstrap,
            AnalysisTriggerPolicy::SimilarityPrepBootstrap,
        ),
        (
            AnalysisTriggerReason::ScanCompleted,
            AnalysisTriggerPolicy::Forbidden,
        ),
        (
            AnalysisTriggerReason::WatcherAutoSync,
            AnalysisTriggerPolicy::Forbidden,
        ),
        (
            AnalysisTriggerReason::DeferredMaintenance,
            AnalysisTriggerPolicy::Forbidden,
        ),
        (
            AnalysisTriggerReason::RenameWithoutContentChange,
            AnalysisTriggerPolicy::Forbidden,
        ),
        (
            AnalysisTriggerReason::SimilarityReadPath,
            AnalysisTriggerPolicy::Forbidden,
        ),
    ];

    for (reason, expected) in cases {
        assert_eq!(reason.policy(), expected, "reason={reason:?}");
    }
}

#[test]
fn manual_selected_source_reanalysis_enqueues_analysis_and_embeddings() {
    let (mut controller, source, _config_dir, _guard) =
        prepare_manual_reanalysis_fixture(&["Pack/a.wav", "Pack/b.wav"]);

    controller.reanalyze_selected_source();

    match wait_for_analysis_message(&mut controller, |message| {
        matches!(
            message,
            analysis_jobs::AnalysisJobMessage::EnqueueFinished { .. }
        )
    }) {
        analysis_jobs::AnalysisJobMessage::EnqueueFinished {
            inserted, announce, ..
        } => {
            assert_eq!(inserted, 2);
            assert!(announce);
        }
        other => panic!("unexpected analysis message: {other:?}"),
    }
    match wait_for_analysis_message(&mut controller, |message| {
        matches!(
            message,
            analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished { .. }
        )
    }) {
        analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
            inserted,
            announce,
            ..
        } => {
            assert_eq!(inserted, 1);
            assert!(announce);
        }
        other => panic!("unexpected embedding message: {other:?}"),
    }
    assert_eq!(
        pending_job_count(&source, analysis_jobs::db::ANALYZE_SAMPLE_JOB_TYPE),
        2
    );
    assert_eq!(
        pending_job_count(&source, analysis_jobs::db::EMBEDDING_BACKFILL_JOB_TYPE),
        1
    );
}

#[test]
fn manual_row_reanalysis_enqueues_only_selected_visible_rows() {
    let (mut controller, source, _config_dir, _guard) =
        prepare_manual_reanalysis_fixture(&["Pack/a.wav", "Pack/b.wav", "Pack/c.wav"]);

    controller
        .reanalyze_browser_rows(&[0, 2, 2, usize::MAX])
        .unwrap();

    match wait_for_analysis_message(&mut controller, |message| {
        matches!(
            message,
            analysis_jobs::AnalysisJobMessage::EnqueueFinished { .. }
        )
    }) {
        analysis_jobs::AnalysisJobMessage::EnqueueFinished {
            inserted, announce, ..
        } => {
            assert_eq!(inserted, 2);
            assert!(announce);
        }
        other => panic!("unexpected analysis message: {other:?}"),
    }
    match wait_for_analysis_message(&mut controller, |message| {
        matches!(
            message,
            analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished { .. }
        )
    }) {
        analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
            inserted,
            announce,
            ..
        } => {
            assert_eq!(inserted, 1);
            assert!(announce);
        }
        other => panic!("unexpected embedding message: {other:?}"),
    }
    assert_eq!(
        pending_job_count(&source, analysis_jobs::db::ANALYZE_SAMPLE_JOB_TYPE),
        2
    );
    assert_eq!(
        pending_job_count(&source, analysis_jobs::db::EMBEDDING_BACKFILL_JOB_TYPE),
        1
    );
}

#[test]
fn canceled_source_remap_allows_analysis_enqueue() {
    let (mut controller, source, _config_dir, _guard) =
        prepare_manual_reanalysis_fixture(&["Pack/a.wav"]);
    controller.runtime.source_lane.pending_remap =
        Some(crate::app::controller::state::runtime::PendingSourceRemap {
            request_id: 91,
            source: source.clone(),
            new_root: tempfile::tempdir().expect("remap destination").keep(),
            queued_at: Instant::now(),
            canceled: true,
        });

    controller.trigger_analysis_for_added_sample(
        &source,
        std::path::Path::new("Pack/a.wav"),
        64,
        1,
    );

    match wait_for_analysis_message(&mut controller, |message| {
        matches!(
            message,
            analysis_jobs::AnalysisJobMessage::EnqueueFinished { .. }
        )
    }) {
        analysis_jobs::AnalysisJobMessage::EnqueueFinished { inserted, .. } => {
            assert_eq!(inserted, 1);
        }
        other => panic!("unexpected analysis message: {other:?}"),
    }
    assert_eq!(
        pending_job_count(&source, analysis_jobs::db::ANALYZE_SAMPLE_JOB_TYPE),
        1
    );
}
