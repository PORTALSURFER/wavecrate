use radiant::gui::panel as panel_ui;
use radiant::widgets::DragHandleMessage;
use std::time::Instant;

use crate::native_app::app::{NativeAppState, emit_gui_action};
use crate::native_app::sample_library::folder_browser::view_contract::{
    MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH,
};

impl NativeAppState {
    pub(in crate::native_app) fn resize_folder_browser(&mut self, message: DragHandleMessage) {
        let started_at = Instant::now();
        let phase = message.phase();
        let should_log = !message.is_moved();
        let outcome = phase.as_str();
        self.ui.chrome.folder_panel.resize(
            message,
            panel_ui::PanelResizeConstraints::right(MIN_FOLDER_WIDTH, MAX_FOLDER_WIDTH),
        );
        if should_log {
            emit_gui_action(
                "layout.resize_folder_browser",
                Some("folder_browser"),
                None,
                outcome,
                started_at,
                None,
            );
        }
    }
}
