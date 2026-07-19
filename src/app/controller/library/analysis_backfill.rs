//! Analysis enqueue and readiness-reconciliation trigger contract.

use super::*;
use std::path::{Path, PathBuf};

mod trigger;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AnalysisTriggerReason {
    SampleAdded,
    AudioContentChanged,
    ScanCompleted,
    WatcherAutoSync,
    DeferredMaintenance,
    RenameWithoutContentChange,
    SimilarityReadPath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AnalysisTriggerPolicy {
    ChangedSamples,
    ReadinessReconciliation,
    Forbidden,
}

impl AnalysisTriggerReason {
    fn policy(self) -> AnalysisTriggerPolicy {
        match self {
            Self::SampleAdded | Self::AudioContentChanged => AnalysisTriggerPolicy::ChangedSamples,
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
}

impl ChangedSampleInput {
    fn new(relative_path: PathBuf, _file_size: u64, _modified_ns: i64) -> Self {
        Self { relative_path }
    }

    fn from_entry(entry: &WavEntry) -> Self {
        Self::new(
            entry.relative_path.clone(),
            entry.file_size,
            entry.modified_ns,
        )
    }
}

enum AnalysisTrigger {
    ChangedSamples {
        source: SampleSource,
        changed_samples: Vec<ChangedSampleInput>,
        announce: bool,
    },
}

impl AnalysisTrigger {
    fn source_id(&self) -> &SourceId {
        match self {
            Self::ChangedSamples { source, .. } => &source.id,
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

    /// Return true if any sources are configured.
    pub fn has_any_sources(&self) -> bool {
        !self.library.sources.is_empty()
    }
}

#[cfg(test)]
mod tests;
