use std::path::Path;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, WaveformState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target as context_menu;
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;

impl NativeAppState {
    pub(in crate::native_app) fn remove_context_source(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Source {
            self.sample_status = String::from("Context target is not a source");
            emit_gui_action(
                "browser.context_menu.source.remove",
                Some("sources"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some("target is not a source"),
            );
            return;
        }
        if !menu.source_removable {
            self.sample_status = String::from("Default source cannot be removed");
            emit_gui_action(
                "browser.context_menu.source.remove",
                Some("sources"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "blocked",
                started_at,
                Some("source not removable"),
            );
            return;
        }
        let Some(source_id) = menu.source_id else {
            self.sample_status = String::from("Source is unavailable");
            emit_gui_action(
                "browser.context_menu.source.remove",
                Some("sources"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some("source unavailable"),
            );
            return;
        };
        let loaded_path = self.waveform.path();
        match self.folder_browser.remove_source(&source_id) {
            Ok(removed) => {
                if path_is_within(&loaded_path, &removed.root) {
                    if let Some(player) = self.audio.player.as_mut() {
                        player.stop();
                    }
                    self.waveform = WaveformState::empty();
                    self.audio.current_playback_span = None;
                }
                self.sample_status = format!("Removed source {}", removed.label);
                self.persist_user_configuration("folder_browser.source.remove.persist", started_at);
                self.sync_source_watcher();
                emit_gui_action(
                    "browser.context_menu.source.remove",
                    Some("sources"),
                    Some(&removed.label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.context_menu.source.remove",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn delete_context_metadata_tag(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::MetadataTag {
            self.sample_status = String::from("Context target is not a tag");
            return;
        }
        let Some(tag) = menu.metadata_tag else {
            self.sample_status = String::from("Tag is unavailable");
            return;
        };
        self.delete_metadata_tag_from_library(tag, context);
    }
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    !path.as_os_str().is_empty() && path.starts_with(root)
}
