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
        self.select_source(id, context);
        self.queue_selected_source_prep(SourcePrepTrigger::SourceSelected, context);
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
