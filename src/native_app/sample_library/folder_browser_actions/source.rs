use radiant::prelude as ui;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::source_prep::SourcePrepTrigger;

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
        log_select_source_phase("select_source", selection_started_at);
        let prep_started_at = Instant::now();
        self.queue_selected_source_prep(SourcePrepTrigger::SourceSelected, context);
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
