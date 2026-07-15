//! Analysis enqueue and readiness-reconciliation trigger contract.

use super::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod trigger;

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
    ReadinessReconciliation,
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
            | Self::RenameWithoutContentChange => AnalysisTriggerPolicy::ReadinessReconciliation,
            Self::SimilarityReadPath => AnalysisTriggerPolicy::Forbidden,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemapConflictPolicy {
    CancelRemap,
    BlockWithStatus,
}

impl AnalysisTrigger {
    fn source_id(&self) -> &SourceId {
        match self {
            Self::ChangedSamples { source, .. } | Self::UserRequestedReanalysis { source, .. } => {
                &source.id
            }
        }
    }

    fn remap_conflict_policy(&self) -> RemapConflictPolicy {
        match self {
            Self::ChangedSamples { .. }
            | Self::UserRequestedReanalysis {
                action: ManualReanalysisAction::SimilarityPrepBootstrap { .. },
                ..
            } => RemapConflictPolicy::CancelRemap,
            Self::UserRequestedReanalysis { .. } => RemapConflictPolicy::BlockWithStatus,
        }
    }
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

    /// Return true if any sources are configured.
    pub fn has_any_sources(&self) -> bool {
        !self.library.sources.is_empty()
    }
}

#[cfg(test)]
mod tests;
