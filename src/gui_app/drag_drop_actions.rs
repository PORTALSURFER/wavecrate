use radiant::prelude as ui;
use radiant::widgets::{DragHandleMessage, DragHandlePhase};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use wavecrate::external_clipboard;

use super::{
    DRAG_PREVIEW_HEIGHT, DRAG_PREVIEW_MAX_WIDTH, FileMoveConflictResolution, FolderBrowserMessage,
    GuiAppState, GuiMessage, emit_gui_action, sample_path_label,
};

impl GuiAppState {
    pub(super) fn drag_sample_file(
        &mut self,
        path: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match drag.phase() {
            DragHandlePhase::Started => {
                self.folder_browser.begin_file_drag(path, drag.position());
                self.arm_browser_drag(context);
            }
            DragHandlePhase::Moved => {
                self.folder_browser.update_drag_pointer(drag.position());
            }
            DragHandlePhase::Ended => {
                self.folder_browser.clear_drag();
                context.end_drag_session();
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.folder_browser.clear_drag();
                context.end_drag_session();
            }
            _ => {}
        }
    }

    pub(super) fn drag_folder(
        &mut self,
        folder_id: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match drag.phase() {
            DragHandlePhase::Started => {
                self.folder_browser
                    .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
                self.arm_browser_drag(context);
            }
            DragHandlePhase::Moved => {
                self.folder_browser
                    .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
            }
            DragHandlePhase::Ended => {
                if let Some(target_folder_id) = self.folder_browser.hovered_drop_target_folder_id()
                {
                    self.drop_browser_drag_on_folder(target_folder_id, context);
                } else {
                    self.folder_browser
                        .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
                    context.end_drag_session();
                }
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.folder_browser.clear_drag();
                context.end_drag_session();
            }
            DragHandlePhase::DoubleActivate => {}
        }
    }

    pub(super) fn drag_waveform_play_selection(
        &mut self,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) -> bool {
        match drag.phase() {
            DragHandlePhase::Started => {
                let started_at = Instant::now();
                match self.extract_waveform_drag_file() {
                    Ok(path) => {
                        self.waveform.flash_play_selection();
                        self.folder_browser
                            .begin_extracted_file_drag(path.clone(), drag.position());
                        self.arm_browser_drag(context);
                        self.sample_status = format!("Dragging {}", sample_path_label(&path));
                        emit_gui_action(
                            "waveform.selection_drag.start",
                            Some("waveform"),
                            None,
                            "success",
                            started_at,
                            None,
                        );
                        true
                    }
                    Err(error) => {
                        self.sample_status = error.clone();
                        emit_gui_action(
                            "waveform.selection_drag.start",
                            Some("waveform"),
                            None,
                            "error",
                            started_at,
                            Some(&error),
                        );
                        false
                    }
                }
            }
            DragHandlePhase::Moved => {
                self.folder_browser.update_drag_pointer(drag.position());
                true
            }
            DragHandlePhase::Ended => {
                self.folder_browser.clear_drag();
                context.end_drag_session();
                true
            }
            DragHandlePhase::Cancelled => {
                self.clear_pending_internal_file_drag_paths();
                self.folder_browser.clear_drag();
                context.end_drag_session();
                true
            }
            _ => false,
        }
    }

    fn extract_waveform_drag_file(&mut self) -> Result<PathBuf, String> {
        let target_folder = self
            .folder_browser
            .selected_folder_path()
            .ok_or_else(|| String::from("Select a folder before dragging a range"))?;
        fs::create_dir_all(&target_folder).map_err(|err| {
            format!(
                "failed to create target folder {}: {err}",
                target_folder.display()
            )
        })?;
        let path = self
            .waveform
            .extract_play_selection_to_folder(&target_folder)?;
        self.folder_browser.refresh_file_path(&path);
        Ok(path)
    }

    pub(super) fn drop_waveform_play_selection_on_sample_list(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let Some(path) = self.folder_browser.extracted_file_drag_path() else {
            return;
        };
        context.end_drag_session();
        self.clear_pending_internal_file_drag_paths();
        self.folder_browser.clear_drag();
        self.folder_browser.refresh_file_path(&path);
        self.sample_status = format!("Extracted {}", sample_path_label(&path));
    }

    fn arm_browser_drag(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let drag = self.folder_browser.drag_preview().map(|preview| {
            ui::DragRequest::new(
                ui::DragPreview::text_sized(
                    preview.label,
                    ui::DragPreviewTextSizing::new(DRAG_PREVIEW_HEIGHT)
                        .min_width(96.0)
                        .max_width(DRAG_PREVIEW_MAX_WIDTH),
                ),
                preview.pointer,
            )
        });
        let external = self.folder_browser.external_drag_request();
        self.arm_pending_internal_file_drag_paths(external.as_ref());

        context.begin_drag_session(drag, external, GuiMessage::ExternalDragCompleted);
    }

    pub(super) fn copy_selected_files(&mut self) {
        let started_at = Instant::now();
        let paths = self.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.sample_status = String::from("Select files before copying");
            emit_gui_action(
                "browser.copy_selected_files",
                Some("browser"),
                None,
                "skipped",
                started_at,
                Some("no selection"),
            );
            return;
        }

        match external_clipboard::copy_file_paths(&paths) {
            Ok(()) => {
                self.sample_status = match paths.len() {
                    1 => String::from("Copied selected file"),
                    count => format!("Copied {count} selected files"),
                };
                emit_gui_action(
                    "browser.copy_selected_files",
                    Some("browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("Copy failed: {error}");
                emit_gui_action(
                    "browser.copy_selected_files",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn external_drag_completed(
        &mut self,
        result: Result<ui::ExternalDragOutcome, String>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        context.end_drag();
        self.folder_browser.clear_drag();
        self.clear_pending_internal_file_drag_paths();
        self.sample_status = match result {
            Ok(outcome) if outcome.accepted() => match outcome.effect {
                ui::ExternalDragEffect::Copy => String::from("Dragged item externally"),
                ui::ExternalDragEffect::Move => String::from("Moved item externally"),
                ui::ExternalDragEffect::Link => String::from("Linked item externally"),
                ui::ExternalDragEffect::None => String::from("External drag cancelled"),
            },
            Ok(_) => String::from("External drag cancelled"),
            Err(error) => format!("External drag failed: {error}"),
        };
    }

    pub(super) fn drop_browser_drag_on_folder(
        &mut self,
        folder_id: String,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.end_drag_session();
        self.clear_pending_internal_file_drag_paths();
        match self.folder_browser.drop_drag_on_folder(&folder_id) {
            Ok(result) => {
                self.apply_moved_sample_paths(&result.moved_paths);
                if let Some(status) = result.status {
                    self.sample_status = status;
                }
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "unchanged"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                self.folder_browser.clear_drag();
                emit_gui_action(
                    "browser.drag_drop.move",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn resolve_file_move_conflict(&mut self, resolution: FileMoveConflictResolution) {
        let started_at = Instant::now();
        match self
            .folder_browser
            .resolve_next_file_move_conflict(resolution)
        {
            Ok(result) => {
                self.apply_moved_sample_paths(&result.moved_paths);
                if let Some(status) = result.status {
                    self.sample_status = status;
                }
                emit_gui_action(
                    "browser.drag_drop.file_conflict.resolve",
                    Some("browser"),
                    None,
                    if result.moved_paths.is_empty() {
                        "skipped"
                    } else {
                        "success"
                    },
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.drag_drop.file_conflict.resolve",
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn cancel_file_move_conflicts(&mut self) {
        if let Some(status) = self.folder_browser.cancel_file_move_conflicts() {
            self.sample_status = status;
        }
    }

    fn apply_moved_sample_paths(&mut self, moved_paths: &[(PathBuf, PathBuf)]) {
        for (old_path, new_path) in moved_paths {
            self.waveform.rewrite_path_prefix(old_path, new_path);
            self.remap_renamed_waveform_cache_path(old_path, new_path);
        }
    }

    pub(super) fn arm_pending_internal_file_drag_paths(
        &mut self,
        request: Option<&ui::ExternalDragRequest>,
    ) {
        self.pending_internal_file_drag_paths.clear();
        let Some(ui::ExternalDragPayload::Files(paths)) = request.map(|request| &request.payload)
        else {
            return;
        };
        self.pending_internal_file_drag_paths
            .extend(paths.iter().map(|path| normalized_drag_path(path)));
    }

    pub(super) fn clear_pending_internal_file_drag_paths(&mut self) {
        self.pending_internal_file_drag_paths.clear();
    }

    pub(super) fn is_pending_internal_file_drag_path(&self, path: &Path) -> bool {
        self.pending_internal_file_drag_paths
            .contains(&normalized_drag_path(path))
    }
}

fn normalized_drag_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
