use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};

impl NativeAppState {
    pub(in crate::native_app) fn clear_rebuildable_caches(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match wavecrate::app_dirs::clear_rebuildable_cache_payloads() {
            Ok(path) => {
                self.audio.settings_error = None;
                self.ui.status.sample = format!("Rebuildable caches cleared: {}", path.display());
                let target = path.display().to_string();
                emit_gui_action(
                    "settings.cache.clear_rebuildable",
                    Some("settings"),
                    Some(target.as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.audio.settings_error = Some(err.clone());
                self.ui.status.sample = err.clone();
                emit_gui_action(
                    "settings.cache.clear_rebuildable",
                    Some("settings"),
                    None,
                    "failed",
                    started_at,
                    Some(err.as_str()),
                );
            }
        }
        self.queue_global_storage_usage_refresh(context);
    }
}
