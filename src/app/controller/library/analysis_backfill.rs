//! Explicit analysis-trigger contract for controller-owned enqueue work.

use super::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnalysisTriggerReason {
    SampleAdded,
    AudioContentChanged,
    UserRequestedReanalysis,
    SimilarityPrepBootstrap,
    ScanCompleted,
    WatcherAutoSync,
    DeferredMaintenance,
    RenameWithoutContentChange,
    SimilarityReadPath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnalysisTriggerPolicy {
    ChangedSamples,
    UserRequestedReanalysis,
    SimilarityPrepBootstrap,
    Forbidden,
}

impl AnalysisTriggerReason {
    fn policy(self) -> AnalysisTriggerPolicy {
        match self {
            Self::SampleAdded | Self::AudioContentChanged => AnalysisTriggerPolicy::ChangedSamples,
            Self::UserRequestedReanalysis => AnalysisTriggerPolicy::UserRequestedReanalysis,
            Self::SimilarityPrepBootstrap => AnalysisTriggerPolicy::SimilarityPrepBootstrap,
            Self::ScanCompleted
            | Self::WatcherAutoSync
            | Self::DeferredMaintenance
            | Self::RenameWithoutContentChange
            | Self::SimilarityReadPath => AnalysisTriggerPolicy::Forbidden,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ChangedSampleInput {
    relative_path: PathBuf,
    file_size: u64,
    modified_ns: i64,
}

impl ChangedSampleInput {
    fn new(relative_path: PathBuf, file_size: u64, modified_ns: i64) -> Self {
        Self {
            relative_path,
            file_size,
            modified_ns,
        }
    }

    fn from_entry(entry: &WavEntry) -> Self {
        Self::new(
            entry.relative_path.clone(),
            entry.file_size,
            entry.modified_ns,
        )
    }

    fn to_changed_sample(&self) -> crate::sample_sources::scanner::ChangedSample {
        crate::sample_sources::scanner::ChangedSample {
            relative_path: self.relative_path.clone(),
            file_size: self.file_size,
            modified_ns: self.modified_ns,
            content_hash: analysis_jobs::fast_content_hash(self.file_size, self.modified_ns),
        }
    }

    fn sample_id(&self, source: &SampleSource) -> String {
        analysis_jobs::build_sample_id(source.id.as_str(), &self.relative_path)
    }
}

#[derive(Debug)]
enum ManualReanalysisAction {
    SelectedSource,
    SelectedRows {
        changed_samples: Vec<ChangedSampleInput>,
        sample_ids: Vec<String>,
    },
    SimilarityPrepBootstrap {
        force_full_analysis: bool,
    },
}

enum AnalysisTrigger {
    ChangedSamples {
        source: SampleSource,
        changed_samples: Vec<ChangedSampleInput>,
        announce: bool,
    },
    UserRequestedReanalysis {
        source: SampleSource,
        action: ManualReanalysisAction,
    },
}

impl AppController {
    /// Enqueue analysis for a newly created sample through the shared trigger contract.
    pub(crate) fn trigger_analysis_for_added_sample(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
    ) {
        debug_assert_eq!(
            AnalysisTriggerReason::SampleAdded.policy(),
            AnalysisTriggerPolicy::ChangedSamples
        );
        self.spawn_analysis_trigger(AnalysisTrigger::ChangedSamples {
            source: source.clone(),
            changed_samples: vec![ChangedSampleInput::new(
                relative_path.to_path_buf(),
                file_size,
                modified_ns,
            )],
            announce: false,
        });
    }

    /// Enqueue analysis for destructive edits that changed one or more samples on disk.
    pub(crate) fn trigger_analysis_for_content_change(
        &mut self,
        source: &SampleSource,
        changed_samples: Vec<ChangedSampleInput>,
        announce: bool,
    ) {
        debug_assert_eq!(
            AnalysisTriggerReason::AudioContentChanged.policy(),
            AnalysisTriggerPolicy::ChangedSamples
        );
        self.spawn_analysis_trigger(AnalysisTrigger::ChangedSamples {
            source: source.clone(),
            changed_samples,
            announce,
        });
    }

    /// Enqueue analysis for one overwritten sample using the shared content-change trigger.
    pub(crate) fn trigger_analysis_for_changed_entry(
        &mut self,
        source: &SampleSource,
        entry: &WavEntry,
        announce: bool,
    ) {
        self.trigger_analysis_for_content_change(
            source,
            vec![ChangedSampleInput::from_entry(entry)],
            announce,
        );
    }

    /// Manually reanalyze the selected source through the explicit replay surface.
    pub fn reanalyze_selected_source(&mut self) {
        let Some(source) = self.current_source() else {
            self.set_status_message(StatusMessage::SelectSourceFirst {
                tone: StatusTone::Warning,
            });
            return;
        };
        self.trigger_manual_reanalysis(source, ManualReanalysisAction::SelectedSource);
    }

    /// Queue analysis jobs to backfill the selected source.
    pub fn backfill_missing_features_for_selected_source(&mut self) {
        self.reanalyze_selected_source();
    }

    /// Queue analysis and embedding replay for the selected source.
    pub fn backfill_embeddings_for_selected_source(&mut self) {
        self.reanalyze_selected_source();
    }

    /// Manually reanalyze selected browser rows by visible index.
    pub fn reanalyze_browser_rows(&mut self, rows: &[usize]) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err("Select a source first".to_string());
        };

        let mut row_samples = BTreeMap::new();
        for &row in rows {
            let Some(entry_index) = self.visible_browser_index(row) else {
                continue;
            };
            let Some(entry) = self.wav_entry(entry_index) else {
                continue;
            };
            let changed = ChangedSampleInput::from_entry(entry);
            row_samples.insert(changed.sample_id(&source), changed);
        }

        if row_samples.is_empty() {
            return Err("No valid samples selected".to_string());
        }

        let (sample_ids, changed_samples): (Vec<_>, Vec<_>) = row_samples.into_iter().unzip();

        self.trigger_manual_reanalysis(
            source,
            ManualReanalysisAction::SelectedRows {
                changed_samples,
                sample_ids,
            },
        );
        Ok(())
    }

    /// Recalculate similarity for the visible browser rows by index.
    pub fn recalc_similarity_for_browser_rows(&mut self, rows: &[usize]) -> Result<(), String> {
        self.reanalyze_browser_rows(rows)
    }

    pub(crate) fn enqueue_similarity_prep_bootstrap(
        &mut self,
        source: SampleSource,
        force_full_analysis: bool,
    ) {
        debug_assert_eq!(
            AnalysisTriggerReason::SimilarityPrepBootstrap.policy(),
            AnalysisTriggerPolicy::SimilarityPrepBootstrap
        );
        self.trigger_manual_reanalysis(
            source,
            ManualReanalysisAction::SimilarityPrepBootstrap {
                force_full_analysis,
            },
        );
    }

    fn trigger_manual_reanalysis(&mut self, source: SampleSource, action: ManualReanalysisAction) {
        debug_assert_eq!(
            AnalysisTriggerReason::UserRequestedReanalysis.policy(),
            AnalysisTriggerPolicy::UserRequestedReanalysis
        );
        self.spawn_analysis_trigger(AnalysisTrigger::UserRequestedReanalysis { source, action });
    }

    fn spawn_analysis_trigger(&mut self, trigger: AnalysisTrigger) {
        let tx = self.runtime.jobs.message_sender();
        std::thread::spawn(move || match trigger {
            AnalysisTrigger::ChangedSamples {
                source,
                changed_samples,
                announce,
            } => {
                let changed_samples: Vec<_> = changed_samples
                    .iter()
                    .map(ChangedSampleInput::to_changed_sample)
                    .collect();
                let result = analysis_jobs::enqueue_jobs_for_source(&source, &changed_samples);
                send_changed_sample_enqueue_result(tx, result, announce);
            }
            AnalysisTrigger::UserRequestedReanalysis { source, action } => match action {
                ManualReanalysisAction::SelectedSource => {
                    let result = analysis_jobs::enqueue_jobs_for_source_backfill_full(&source);
                    send_changed_sample_enqueue_result(tx.clone(), result, true);

                    let result = analysis_jobs::enqueue_jobs_for_embedding_backfill(&source);
                    send_embedding_enqueue_result(tx, result, true);
                }
                ManualReanalysisAction::SelectedRows {
                    changed_samples,
                    sample_ids,
                } => {
                    if !changed_samples.is_empty() {
                        let changed_samples: Vec<_> = changed_samples
                            .iter()
                            .map(ChangedSampleInput::to_changed_sample)
                            .collect();
                        let result =
                            analysis_jobs::enqueue_jobs_for_source(&source, &changed_samples);
                        send_changed_sample_enqueue_result(tx.clone(), result, true);
                    }
                    let result =
                        analysis_jobs::enqueue_jobs_for_embedding_samples(&source, &sample_ids);
                    send_embedding_enqueue_result(tx, result, true);
                }
                ManualReanalysisAction::SimilarityPrepBootstrap {
                    force_full_analysis,
                } => {
                    let analysis_result = if force_full_analysis {
                        analysis_jobs::enqueue_jobs_for_source_backfill_full(&source)
                    } else {
                        analysis_jobs::enqueue_jobs_for_source_backfill(&source)
                    };
                    send_changed_sample_enqueue_result(tx.clone(), analysis_result, true);

                    let embed_result = analysis_jobs::enqueue_jobs_for_embedding_backfill(&source);
                    send_embedding_enqueue_result(tx, embed_result, true);
                }
            },
        });
    }

    /// Return true if any sources are configured.
    pub fn has_any_sources(&self) -> bool {
        !self.library.sources.is_empty()
    }
}

fn send_changed_sample_enqueue_result(
    tx: super::jobs::JobMessageSender,
    result: Result<(usize, analysis_jobs::AnalysisProgress), String>,
    announce: bool,
) {
    match result {
        Ok((inserted, progress)) => {
            let _ = tx.send(super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EnqueueFinished {
                    inserted,
                    progress,
                    announce,
                },
            ));
        }
        Err(err) => {
            let _ = tx.send(super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EnqueueFailed(err),
            ));
        }
    }
}

fn send_embedding_enqueue_result(
    tx: super::jobs::JobMessageSender,
    result: Result<(usize, analysis_jobs::AnalysisProgress), String>,
    announce: bool,
) {
    match result {
        Ok((inserted, progress)) => {
            let _ = tx.send(super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
                    inserted,
                    progress,
                    announce,
                },
            ));
        }
        Err(err) => {
            let _ = tx.send(super::jobs::JobMessage::Analysis(
                analysis_jobs::AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
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
}
