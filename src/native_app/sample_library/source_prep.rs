use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum ReadinessIntent {
    Preserve,
    RequestConvergence,
    InvalidateAndRequestConvergence,
    Reanalyze,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourcePriorityIntent {
    PromoteIfSelected,
    PromoteSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum MetadataRefreshIntent {
    IfNotLoaded,
    Force,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum CacheWarmIntent {
    Preserve,
    SelectedFolderOrSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourceFeedbackIntent {
    Preserve,
    QueuedIfCacheWarmNotScheduled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SourcePrepIntents {
    pub(in crate::native_app) readiness: ReadinessIntent,
    pub(in crate::native_app) priority: SourcePriorityIntent,
    pub(in crate::native_app) metadata_refresh: MetadataRefreshIntent,
    pub(in crate::native_app) refresh_waveform_cache_projection_if_selected: bool,
    pub(in crate::native_app) cache_warm: CacheWarmIntent,
    pub(in crate::native_app) feedback: SourceFeedbackIntent,
}

impl NativeAppState {
    pub(in crate::native_app) fn queue_selected_source_prep(
        &mut self,
        intents: SourcePrepIntents,
        reason: &'static str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let source_id = self.library.folder_browser.selected_source_id().to_string();
        self.queue_source_prep(source_id, intents, reason, context);
    }

    pub(in crate::native_app) fn queue_source_prep(
        &mut self,
        source_id: String,
        intents: SourcePrepIntents,
        reason: &'static str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let selected_source = source_id == self.library.folder_browser.selected_source_id();
        match intents.priority {
            SourcePriorityIntent::PromoteIfSelected if selected_source => self
                .background
                .source_processing
                .set_selected_source(Some(&source_id)),
            SourcePriorityIntent::PromoteSource => self
                .background
                .source_processing
                .set_selected_source(Some(&source_id)),
            SourcePriorityIntent::PromoteIfSelected => {}
        }
        match intents.readiness {
            ReadinessIntent::Preserve => {}
            ReadinessIntent::RequestConvergence => self
                .background
                .source_processing
                .request_source_processing(&source_id, reason),
            ReadinessIntent::InvalidateAndRequestConvergence => self
                .background
                .source_processing
                .wake_source(&source_id, reason),
            ReadinessIntent::Reanalyze => self
                .background
                .source_processing
                .request_source_reanalysis(&source_id, reason),
        }
        self.schedule_persisted_metadata_tags_refresh_for_source(
            &source_id,
            intents.metadata_refresh == MetadataRefreshIntent::Force,
            context,
        );
        if selected_source && intents.refresh_waveform_cache_projection_if_selected {
            self.schedule_persisted_waveform_cache_indicator_refresh(context);
        }
        let cache_scheduled = match intents.cache_warm {
            CacheWarmIntent::Preserve => false,
            CacheWarmIntent::SelectedFolderOrSource if selected_source => {
                self.schedule_active_folder_cache_warm(context)
            }
            CacheWarmIntent::SelectedFolderOrSource => {
                self.schedule_source_cache_warm(&source_id, context)
            }
        };
        if intents.feedback == SourceFeedbackIntent::QueuedIfCacheWarmNotScheduled
            && !cache_scheduled
        {
            self.ui.status.sample = String::from("Source processing queued");
        }
        emit_gui_action(
            "source_prep.queue",
            Some("background"),
            Some(&source_id),
            reason,
            started_at,
            None,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CacheWarmIntent, MetadataRefreshIntent, ReadinessIntent, SourceFeedbackIntent,
        SourcePrepIntents, SourcePriorityIntent,
    };
    use crate::native_app::sample_library::{
        committed_file_mutations::{
            COMMITTED_MUTATION_PREP_INTENTS, COMMITTED_MUTATION_PREP_REASON,
        },
        folder_browser_actions::{
            FOLDER_ACTIVATION_PREP_INTENTS, FOLDER_ACTIVATION_PREP_REASON,
            SOURCE_SELECTION_PREP_INTENTS, SOURCE_SELECTION_PREP_REASON,
        },
        folder_scan_actions::{
            FILESYSTEM_SYNC_PREP_INTENTS, FILESYSTEM_SYNC_PREP_REASON, PROCESS_SOURCE_PREP_INTENTS,
            PROCESS_SOURCE_PREP_REASON, SOURCE_SCAN_COMPLETION_PREP_INTENTS,
            SOURCE_SCAN_COMPLETION_PREP_REASON,
        },
        folder_startup_verify_actions::{
            VERIFIED_FOLDER_MUTATION_PREP_INTENTS, VERIFIED_FOLDER_MUTATION_PREP_REASON,
        },
        folder_tree_refresh_actions::{VERIFIED_SOURCE_PREP_INTENTS, VERIFIED_SOURCE_PREP_REASON},
    };

    const PASSIVE_SELECTION: SourcePrepIntents = SourcePrepIntents {
        readiness: ReadinessIntent::RequestConvergence,
        priority: SourcePriorityIntent::PromoteIfSelected,
        metadata_refresh: MetadataRefreshIntent::IfNotLoaded,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::Preserve,
        feedback: SourceFeedbackIntent::Preserve,
    };
    const SCAN_COMPLETION: SourcePrepIntents = SourcePrepIntents {
        readiness: ReadinessIntent::Preserve,
        priority: SourcePriorityIntent::PromoteIfSelected,
        metadata_refresh: MetadataRefreshIntent::Force,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::Preserve,
        feedback: SourceFeedbackIntent::Preserve,
    };
    const FILESYSTEM_MUTATION: SourcePrepIntents = SourcePrepIntents {
        readiness: ReadinessIntent::InvalidateAndRequestConvergence,
        priority: SourcePriorityIntent::PromoteIfSelected,
        metadata_refresh: MetadataRefreshIntent::Force,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::Preserve,
        feedback: SourceFeedbackIntent::Preserve,
    };
    const EXPLICIT_PROCESS_SOURCE: SourcePrepIntents = SourcePrepIntents {
        readiness: ReadinessIntent::Reanalyze,
        priority: SourcePriorityIntent::PromoteSource,
        metadata_refresh: MetadataRefreshIntent::Force,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::SelectedFolderOrSource,
        feedback: SourceFeedbackIntent::QueuedIfCacheWarmNotScheduled,
    };

    #[test]
    fn call_site_matrix_declares_exact_source_prep_effects() {
        let scenarios = [
            (
                "source selection",
                SOURCE_SELECTION_PREP_INTENTS,
                SOURCE_SELECTION_PREP_REASON,
                PASSIVE_SELECTION,
                "source_selected",
            ),
            (
                "folder activation",
                FOLDER_ACTIVATION_PREP_INTENTS,
                FOLDER_ACTIVATION_PREP_REASON,
                PASSIVE_SELECTION,
                "folder_activated",
            ),
            (
                "verified startup",
                VERIFIED_SOURCE_PREP_INTENTS,
                VERIFIED_SOURCE_PREP_REASON,
                PASSIVE_SELECTION,
                "source_verified",
            ),
            (
                "scan completion",
                SOURCE_SCAN_COMPLETION_PREP_INTENTS,
                SOURCE_SCAN_COMPLETION_PREP_REASON,
                SCAN_COMPLETION,
                "source_scan_finished",
            ),
            (
                "verified folder mutation",
                VERIFIED_FOLDER_MUTATION_PREP_INTENTS,
                VERIFIED_FOLDER_MUTATION_PREP_REASON,
                FILESYSTEM_MUTATION,
                "filesystem_changed",
            ),
            (
                "external filesystem sync",
                FILESYSTEM_SYNC_PREP_INTENTS,
                FILESYSTEM_SYNC_PREP_REASON,
                FILESYSTEM_MUTATION,
                "filesystem_changed",
            ),
            (
                "committed Wavecrate mutation",
                COMMITTED_MUTATION_PREP_INTENTS,
                COMMITTED_MUTATION_PREP_REASON,
                FILESYSTEM_MUTATION,
                "filesystem_changed",
            ),
            (
                "explicit Process Source",
                PROCESS_SOURCE_PREP_INTENTS,
                PROCESS_SOURCE_PREP_REASON,
                EXPLICIT_PROCESS_SOURCE,
                "user_requested",
            ),
        ];

        for (scenario, actual_intents, actual_reason, expected_intents, expected_reason) in
            scenarios
        {
            assert_eq!(
                actual_intents, expected_intents,
                "{scenario} must request exactly its documented effects"
            );
            assert_eq!(
                actual_reason, expected_reason,
                "{scenario} must retain its descriptive telemetry reason"
            );
        }
    }

    #[test]
    fn descriptive_reasons_are_independent_from_effect_policy() {
        assert_ne!(
            SOURCE_SELECTION_PREP_REASON, FOLDER_ACTIVATION_PREP_REASON,
            "distinct events retain distinct telemetry reasons"
        );
        assert_eq!(
            SOURCE_SELECTION_PREP_INTENTS, FOLDER_ACTIVATION_PREP_INTENTS,
            "different reasons may deliberately request identical effects"
        );
        assert_eq!(
            VERIFIED_FOLDER_MUTATION_PREP_REASON,
            FILESYSTEM_SYNC_PREP_REASON
        );
        assert_eq!(
            VERIFIED_FOLDER_MUTATION_PREP_INTENTS, FILESYSTEM_SYNC_PREP_INTENTS,
            "shared reason text does not hide caller-owned policy"
        );
        assert_ne!(
            SOURCE_SELECTION_PREP_INTENTS, SOURCE_SCAN_COMPLETION_PREP_INTENTS,
            "metadata refresh policy is explicit even when readiness policy matches"
        );
    }
}
