use radiant::prelude as ui;
use std::time::{Duration, Instant};

use crate::native_app::app_scope::{GuiMessage, NativeAppState, emit_gui_action, logging};
use crate::native_app::browser::folder_browser::RenamePathRemap;

impl NativeAppState {
    pub(in crate::native_app) fn begin_folder_browser_rename(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        let target = self.folder_browser.selected_rename_target();
        if logging::debug_logging_enabled() {
            tracing::debug!(
                target: logging::ACTION_EVENT_TARGET,
                event = "action_detail",
                action = "folder_browser.rename.begin",
                pane = "folder_browser",
                target_kind = target.kind,
                target_label = target.label,
                is_source_root = target.is_source_root,
                "Folder browser rename requested"
            );
        }
        let renaming_file = self.folder_browser.selected_file_id().is_some();
        match self.folder_browser.begin_rename_selected() {
            Ok(Some(input_id)) => {
                self.sample_status = if renaming_file {
                    String::from("Renaming selected file")
                } else {
                    String::from("Renaming selected folder")
                };
                context.after(
                    Duration::from_millis(1),
                    GuiMessage::FocusRenameInput(input_id),
                );
                emit_gui_action(
                    "folder_browser.rename.begin",
                    Some("folder_browser"),
                    Some(target.kind),
                    "success",
                    started_at,
                    None,
                );
            }
            Ok(None) => {
                self.sample_status = String::from("Select a folder to rename");
                emit_gui_action(
                    "folder_browser.rename.begin",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some("nothing_selected"),
                );
            }
            Err(error) => {
                self.sample_status = error;
                emit_gui_action(
                    "folder_browser.rename.begin",
                    Some("folder_browser"),
                    None,
                    "error",
                    started_at,
                    Some("rename_begin_failed"),
                );
            }
        }
    }

    pub(in crate::native_app) fn begin_folder_browser_subfolder_creation(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        match self.folder_browser.begin_create_subfolder() {
            Ok(Some(input_id)) => {
                self.sample_status = String::from("Creating new folder");
                context.after(
                    Duration::from_millis(1),
                    GuiMessage::FocusRenameInput(input_id),
                );
                emit_gui_action(
                    "folder_browser.folder.create_begin",
                    Some("folder_browser"),
                    Some("folder"),
                    "success",
                    started_at,
                    None,
                );
            }
            Ok(None) => {
                self.sample_status = String::from("Select a folder to add a subfolder");
                emit_gui_action(
                    "folder_browser.folder.create_begin",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some("nothing_selected"),
                );
            }
            Err(error) => {
                self.sample_status = error;
                emit_gui_action(
                    "folder_browser.folder.create_begin",
                    Some("folder_browser"),
                    None,
                    "error",
                    started_at,
                    Some("create_begin_failed"),
                );
            }
        }
    }

    pub(in crate::native_app) fn apply_folder_browser_rename_input(
        &mut self,
        message: radiant::widgets::TextInputMessage,
    ) {
        let started_at = Instant::now();
        let input_action = rename_input_action(&message);
        if let Some(result) = self.folder_browser.apply_rename_input(message) {
            if let Some(remap) = result.path_remap {
                self.apply_browser_rename_path_remap(&remap);
            }
            self.sample_status = result.status;
        }
        if let Some(action) = input_action {
            emit_gui_action(
                action,
                Some("folder_browser"),
                None,
                "applied",
                started_at,
                None,
            );
        }
    }

    fn apply_browser_rename_path_remap(&mut self, remap: &RenamePathRemap) {
        self.waveform
            .rewrite_path_prefix(&remap.old_path, &remap.new_path);
        self.remap_renamed_waveform_cache_path(&remap.old_path, &remap.new_path);
    }
}

fn rename_input_action(message: &radiant::widgets::TextInputMessage) -> Option<&'static str> {
    message
        .is_submitted()
        .then_some("folder_browser.rename.submit")
}
