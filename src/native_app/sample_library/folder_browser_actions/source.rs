use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::source_prep::{
    CacheWarmIntent, MetadataRefreshIntent, ReadinessIntent, SourceFeedbackIntent,
    SourcePrepIntents, SourcePriorityIntent,
};

pub(in crate::native_app) const SOURCE_SELECTION_PREP_INTENTS: SourcePrepIntents =
    SourcePrepIntents {
        readiness: ReadinessIntent::RequestConvergence,
        priority: SourcePriorityIntent::PromoteIfSelected,
        metadata_refresh: MetadataRefreshIntent::IfNotLoaded,
        refresh_waveform_cache_projection_if_selected: true,
        cache_warm: CacheWarmIntent::Preserve,
        feedback: SourceFeedbackIntent::Preserve,
    };
pub(in crate::native_app) const SOURCE_SELECTION_PREP_REASON: &str = "source_selected";

impl NativeAppState {
    pub(super) fn select_folder_browser_source(
        &mut self,
        id: String,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let source = id.clone();
        self.ui.browser_interaction.context_menu = None;
        let selection_started_at = Instant::now();
        self.select_source(id, context);
        if self.library.folder_browser.selected_source_id() == source {
            self.library
                .folder_browser
                .focus_selected_source_for_keyboard();
        }
        log_select_source_phase("select_source", selection_started_at);
        let prep_started_at = Instant::now();
        self.queue_selected_source_prep(
            SOURCE_SELECTION_PREP_INTENTS,
            SOURCE_SELECTION_PREP_REASON,
            context,
        );
        log_select_source_phase("queue_selected_source_prep", prep_started_at);
        emit_gui_action(
            "folder_browser.select_source",
            Some("folder_browser"),
            Some(source.as_str()),
            "applied",
            started_at,
            None,
        );
    }

    pub(super) fn navigate_folder_browser_source(
        &mut self,
        delta: i32,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(source_id) = self.library.folder_browser.adjacent_source_id(delta) else {
            return;
        };
        self.select_folder_browser_source(source_id, context);
    }
}

fn log_select_source_phase(phase: &'static str, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed.as_millis() >= 4 {
        tracing::warn!(
            target: "wavecrate::debug::ui_frame",
            event = "folder_browser.select_source.phase",
            phase,
            elapsed_ms = elapsed.as_secs_f64() * 1000.0,
            "slow folder browser source selection phase"
        );
    }
}
