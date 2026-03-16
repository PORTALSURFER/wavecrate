use super::progress;
use super::*;
use crate::app::state::ProgressTaskKind;

/// Apply incremental scan progress to the shared progress UI.
pub(crate) fn handle_scan_progress(
    controller: &mut AppController,
    completed: usize,
    detail: Option<String>,
) {
    let detail = match detail {
        Some(detail) if !detail.is_empty() => {
            format!("Scanned {completed} file(s)\n{detail}")
        }
        _ => format!("Scanned {completed} file(s)"),
    };
    progress::update_progress_detail(controller, ProgressTaskKind::Scan, completed, Some(detail));
}

/// Finalize scan state, refresh caches, and queue analysis follow-up work.
pub(crate) fn handle_scan_finished(controller: &mut AppController, result: ScanResult) {
    controller.runtime.jobs.clear_scan();
    if controller.ui.progress.task == Some(ProgressTaskKind::Scan) {
        controller.clear_progress();
    }
    let is_selected_source =
        Some(&result.source_id) == controller.selection_state.ctx.selected_source.as_ref();
    let is_auto = matches!(result.kind, ScanKind::Auto);
    let label = match result.mode {
        ScanMode::Quick => "Quick sync",
        ScanMode::Hard => "Hard sync",
    };
    match result.result {
        Ok(stats) => {
            let changed_samples = stats.changed_samples.clone();
            let scan_changed = !changed_samples.is_empty();
            let similarity_prep_active = controller
                .runtime
                .similarity_prep
                .as_ref()
                .is_some_and(|state| state.source_id == result.source_id);
            if is_selected_source && (!is_auto || scan_changed) {
                controller.set_status(
                    format!(
                        "{label} complete: {} added, {} updated, {} missing",
                        stats.added, stats.updated, stats.missing
                    ),
                    StatusTone::Info,
                );
            }

            {
                let mut invalidator =
                    source_cache_invalidator::SourceCacheInvalidator::new_from_state(
                        &mut controller.cache,
                        &mut controller.ui_cache,
                        &mut controller.library.missing,
                    );
                invalidator.invalidate_wav_related(&result.source_id);
            }

            if is_selected_source {
                controller.queue_wav_load();
            }

            let source_for_jobs = controller
                .library
                .sources
                .iter()
                .find(|source| source.id == result.source_id)
                .cloned();
            let source_for_duration = source_for_jobs.clone();

            if scan_changed {
                if let Some(source) = source_for_jobs.clone() {
                    let tx = controller.runtime.jobs.message_sender();
                    let changed_samples = changed_samples.clone();
                    std::thread::spawn(move || {
                        let result =
                            analysis_jobs::enqueue_jobs_for_source(&source, &changed_samples);
                        match result {
                            Ok((inserted, progress)) => {
                                let _ = tx.send(JobMessage::Analysis(
                                    super::AnalysisJobMessage::EnqueueFinished {
                                        inserted,
                                        progress,
                                    },
                                ));
                            }
                            Err(err) => {
                                let _ = tx.send(JobMessage::Analysis(
                                    super::AnalysisJobMessage::EnqueueFailed(err),
                                ));
                            }
                        }
                    });
                }
            } else if let Some(source) = source_for_jobs.clone() {
                if similarity_prep_active {
                    controller.handle_similarity_scan_finished(&result.source_id, false);
                    return;
                }
                let tx = controller.runtime.jobs.message_sender();
                std::thread::spawn(move || {
                    let result = analysis_jobs::enqueue_jobs_for_source_backfill(&source);
                    match result {
                        Ok((inserted, progress)) => {
                            let _ = tx.send(JobMessage::Analysis(
                                super::AnalysisJobMessage::EnqueueFinished { inserted, progress },
                            ));
                        }
                        Err(err) => {
                            let _ = tx.send(JobMessage::Analysis(
                                super::AnalysisJobMessage::EnqueueFailed(err),
                            ));
                        }
                    }
                    let embed_result = analysis_jobs::enqueue_jobs_for_embedding_backfill(&source);
                    match embed_result {
                        Ok((inserted, progress)) => {
                            if inserted > 0 {
                                let _ = tx.send(JobMessage::Analysis(
                                    super::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
                                        inserted,
                                        progress,
                                    },
                                ));
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(JobMessage::Analysis(
                                super::AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err),
                            ));
                        }
                    }
                });
            }
            if let Some(source) = source_for_duration {
                let tx = controller.runtime.jobs.message_sender();
                std::thread::spawn(
                    move || match analysis_jobs::update_missing_durations_for_source(&source) {
                        Ok(updated) => {
                            if updated > 0 {
                                let _ = tx.send(JobMessage::Analysis(
                                    super::AnalysisJobMessage::DurationsUpdated {
                                        source_id: source.id.clone(),
                                        updated,
                                    },
                                ));
                            }
                        }
                        Err(err) => {
                            tracing::warn!(
                                "Duration probe after scan failed for {}: {err}",
                                source.id.as_str()
                            );
                        }
                    },
                );
            }
            controller.handle_similarity_scan_finished(&result.source_id, scan_changed);
        }
        Err(crate::sample_sources::scanner::ScanError::Canceled) => {
            if is_selected_source {
                controller.set_status(format!("{label} canceled"), StatusTone::Warning);
            }
            controller.cancel_similarity_prep(&result.source_id);
        }
        Err(err) => {
            if is_selected_source {
                controller.set_status(format!("{label} failed: {err}"), StatusTone::Error);
            }
            controller.cancel_similarity_prep(&result.source_id);
        }
    }
}

#[cfg(test)]
mod tests {
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
            assert!(Instant::now() < deadline, "timed out waiting for analysis message");
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
        assert!(!controller.ui_cache.browser.features.contains_key(&source.id));
        assert!(controller.wav_entries.source_id.as_ref() == Some(&source.id));
        assert_eq!(controller.wav_entries.total, 1);

        match wait_for_analysis_message(&mut controller, |message| {
            matches!(message, AnalysisJobMessage::EnqueueFinished { .. })
        }) {
            AnalysisJobMessage::EnqueueFinished { inserted: _, progress: _ } => {}
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
}
