use std::path::Path;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, WaveformState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target as context_menu;
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;
use wavecrate::sample_sources::SourceRole;

impl NativeAppState {
    pub(in crate::native_app) fn toggle_context_folder_lock(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Folder {
            self.ui.status.sample = String::from("Choose a folder to lock");
            emit_gui_action(
                "browser.context_menu.folder.lock",
                Some("folder_browser"),
                None,
                "blocked",
                started_at,
                Some("unsupported target"),
            );
            return;
        }

        let folder_id = menu.path.to_string_lossy().to_string();
        match self.library.folder_browser.toggle_folder_lock(&folder_id) {
            Ok(true) => {
                let label = context_menu::target_label(&menu.path);
                self.ui.status.sample = format!("Locked folder {label}");
                self.persist_user_configuration("folder_browser.folder.lock.persist", started_at);
                emit_gui_action(
                    "browser.context_menu.folder.lock",
                    Some("folder_browser"),
                    Some(label.as_str()),
                    "locked",
                    started_at,
                    None,
                );
            }
            Ok(false) => {
                let label = context_menu::target_label(&menu.path);
                self.ui.status.sample = format!("Unlocked folder {label}");
                self.persist_user_configuration("folder_browser.folder.lock.persist", started_at);
                emit_gui_action(
                    "browser.context_menu.folder.lock",
                    Some("folder_browser"),
                    Some(label.as_str()),
                    "unlocked",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.folder.lock",
                    Some("folder_browser"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn remove_context_source(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Source {
            self.ui.status.sample = String::from("Context target is not a source");
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
            self.ui.status.sample = String::from("Default source cannot be removed");
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
            self.ui.status.sample = String::from("Source is unavailable");
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
        let loaded_path = self.waveform.current.path();
        match self.library.folder_browser.remove_source(&source_id) {
            Ok(removed) => {
                if path_is_within(&loaded_path, &removed.root) {
                    self.stop_audio_output_playback();
                    self.waveform.current = WaveformState::empty();
                    self.audio.current_playback_span = None;
                }
                self.ui.status.sample = format!("Removed source {}", removed.label);
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
                self.ui.status.sample = error.clone();
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

    pub(in crate::native_app) fn toggle_context_source_protection(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Source {
            self.ui.status.sample = String::from("Context target is not a source");
            return;
        }
        let Some(source_id) = menu.source_id else {
            self.ui.status.sample = String::from("Source is unavailable");
            return;
        };
        let protect = menu.source_role != SourceRole::Protected;
        match self
            .library
            .folder_browser
            .set_source_protected(&source_id, protect)
        {
            Ok(status) => {
                self.ui.status.sample = status.to_string();
                self.persist_user_configuration("folder_browser.source.role.persist", started_at);
                emit_gui_action(
                    "browser.context_menu.source.protection",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    if protect { "protected" } else { "unprotected" },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.source.protection",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn set_context_source_primary(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Source {
            self.ui.status.sample = String::from("Context target is not a source");
            return;
        }
        let Some(source_id) = menu.source_id else {
            self.ui.status.sample = String::from("Source is unavailable");
            return;
        };
        match self.library.folder_browser.set_primary_source(&source_id) {
            Ok(status) => {
                self.ui.status.sample = status.to_string();
                self.persist_user_configuration("folder_browser.source.role.persist", started_at);
                emit_gui_action(
                    "browser.context_menu.source.primary",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "primary",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.source.primary",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn clear_context_source_primary(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Source {
            self.ui.status.sample = String::from("Context target is not a source");
            return;
        }
        let Some(source_id) = menu.source_id else {
            self.ui.status.sample = String::from("Source is unavailable");
            return;
        };
        match self.library.folder_browser.clear_primary_source(&source_id) {
            Ok(status) => {
                self.ui.status.sample = status.to_string();
                self.persist_user_configuration("folder_browser.source.role.persist", started_at);
                emit_gui_action(
                    "browser.context_menu.source.primary",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "cleared",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.source.primary",
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
        context: &mut radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::MetadataTag {
            self.ui.status.sample = String::from("Context target is not a tag");
            return;
        }
        let Some(tag) = menu.metadata_tag else {
            self.ui.status.sample = String::from("Tag is unavailable");
            return;
        };
        self.delete_metadata_tag_from_library(tag, context);
    }
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    !path.as_os_str().is_empty() && path.starts_with(root)
}
