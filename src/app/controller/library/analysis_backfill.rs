//! Explicit analysis-trigger contract for controller-owned enqueue work.

use super::*;
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

enum ExplicitReanalysisRequest {
    MissingFeaturesForSource,
    EmbeddingsForSource,
    Samples {
        changed_samples: Vec<ChangedSampleInput>,
        sample_ids: Vec<String>,
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
        request: ExplicitReanalysisRequest,
    },
    SimilarityPrepBootstrap {
        source: SampleSource,
        force_full_analysis: bool,
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

    /// Queue analysis jobs to backfill missing features for the selected source.
    pub fn backfill_missing_features_for_selected_source(&mut self) {
        let Some(source) = self.current_source() else {
            self.set_status_message(StatusMessage::SelectSourceFirst {
                tone: StatusTone::Warning,
            });
            return;
        };
        debug_assert_eq!(
            AnalysisTriggerReason::UserRequestedReanalysis.policy(),
            AnalysisTriggerPolicy::UserRequestedReanalysis
        );
        self.spawn_analysis_trigger(AnalysisTrigger::UserRequestedReanalysis {
            source,
            request: ExplicitReanalysisRequest::MissingFeaturesForSource,
        });
    }

    /// Queue embedding jobs for the selected source through the explicit trigger contract.
    pub fn backfill_embeddings_for_selected_source(&mut self) {
        let Some(source) = self.current_source() else {
            self.set_status_message(StatusMessage::SelectSourceFirst {
                tone: StatusTone::Warning,
            });
            return;
        };
        debug_assert_eq!(
            AnalysisTriggerReason::UserRequestedReanalysis.policy(),
            AnalysisTriggerPolicy::UserRequestedReanalysis
        );
        self.spawn_analysis_trigger(AnalysisTrigger::UserRequestedReanalysis {
            source,
            request: ExplicitReanalysisRequest::EmbeddingsForSource,
        });
    }

    /// Recalculate similarity for the visible browser rows by index.
    pub fn recalc_similarity_for_browser_rows(&mut self, rows: &[usize]) -> Result<(), String> {
        let Some(source) = self.current_source() else {
            return Err("Select a source first".to_string());
        };

        let mut changed_samples = Vec::new();
        let mut sample_ids = Vec::new();
        for &row in rows {
            let Some(entry_index) = self.visible_browser_index(row) else {
                continue;
            };
            let Some(entry) = self.wav_entry(entry_index) else {
                continue;
            };
            let changed = ChangedSampleInput::from_entry(entry);
            sample_ids.push(changed.sample_id(&source));
            changed_samples.push(changed);
        }

        if sample_ids.is_empty() {
            return Err("No valid samples selected".to_string());
        }

        sample_ids.sort();
        sample_ids.dedup();

        debug_assert_eq!(
            AnalysisTriggerReason::UserRequestedReanalysis.policy(),
            AnalysisTriggerPolicy::UserRequestedReanalysis
        );
        self.spawn_analysis_trigger(AnalysisTrigger::UserRequestedReanalysis {
            source,
            request: ExplicitReanalysisRequest::Samples {
                changed_samples,
                sample_ids,
            },
        });
        Ok(())
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
        self.spawn_analysis_trigger(AnalysisTrigger::SimilarityPrepBootstrap {
            source,
            force_full_analysis,
        });
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
            AnalysisTrigger::UserRequestedReanalysis { source, request } => match request {
                ExplicitReanalysisRequest::MissingFeaturesForSource => {
                    let result = analysis_jobs::enqueue_jobs_for_source_missing_features(&source);
                    send_changed_sample_enqueue_result(tx, result, true);
                }
                ExplicitReanalysisRequest::EmbeddingsForSource => {
                    let result = analysis_jobs::enqueue_jobs_for_embedding_backfill(&source);
                    send_embedding_enqueue_result(tx, result, true);
                }
                ExplicitReanalysisRequest::Samples {
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
            },
            AnalysisTrigger::SimilarityPrepBootstrap {
                source,
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
}
