use std::{path::PathBuf, time::Instant};

use radiant::prelude as ui;
use wavecrate::{external_clipboard, selection::SelectionRange};

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};

mod clipboard_clip;

const CLIPBOARD_HANDOFF_TASK_NAME: &str = "gui-copy-selected-files";
const WAVEFORM_CLIPBOARD_HANDOFF_TASK_NAME: &str = "gui-copy-waveform-selection";

impl NativeAppState {
    pub(in crate::native_app) fn copy_selected_files(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.copy_waveform_play_selection_if_marked(started_at, context) {
            return;
        }

        let paths = self.library.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.ui.status.sample = String::from("Select files before copying");
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

        let count = paths.len();
        self.yield_sample_cache_warm_for_user_handoff(context);
        self.library.folder_browser.flash_copied_file_paths(&paths);
        self.ui.status.sample = match count {
            1 => String::from("Copying selected file"),
            count => format!("Copying {count} selected files"),
        };
        context
            .business()
            .interactive(CLIPBOARD_HANDOFF_TASK_NAME)
            .run(
                move |worker_context| {
                    worker_context.checkpoint()?;
                    external_clipboard::copy_file_paths(&paths).map_err(|error| error.to_string())
                },
                move |result| GuiMessage::SelectedFilesCopyFinished {
                    count,
                    started_at,
                    result,
                },
            );
    }

    pub(in crate::native_app) fn finish_copy_selected_files(
        &mut self,
        count: usize,
        started_at: Instant,
        result: Result<(), String>,
    ) {
        match result {
            Ok(()) => {
                self.ui.status.sample = match count {
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
                self.ui.status.sample = format!("Copy failed: {error}");
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

    fn copy_waveform_play_selection_if_marked(
        &mut self,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) -> bool {
        if self.waveform.current.play_selection().is_none() {
            return false;
        }
        let request = match self
            .waveform
            .current
            .play_selection_extraction_request(None)
        {
            Ok(request) => request,
            Err(error) => {
                self.ui.status.sample = format!("Copy failed: {error}");
                emit_gui_action(
                    "waveform.copy_playmarked_range",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
                return true;
            }
        };
        let source_path = request.source_path().to_path_buf();
        let selection = request.selection();
        self.waveform.current.flash_play_selection();
        self.yield_sample_cache_warm_for_user_handoff(context);
        self.ui.status.sample = String::from("Copying play range");
        context
            .business()
            .interactive(WAVEFORM_CLIPBOARD_HANDOFF_TASK_NAME)
            .run(
                move |worker_context| {
                    clipboard_clip::copy_waveform_selection_clip_to_clipboard(
                        worker_context,
                        request,
                    )
                },
                move |result| GuiMessage::WaveformSelectionCopyFinished {
                    source_path,
                    selection,
                    started_at,
                    result,
                },
            );
        true
    }

    pub(in crate::native_app) fn finish_waveform_selection_copy(
        &mut self,
        source_path: PathBuf,
        selection: SelectionRange,
        started_at: Instant,
        result: Result<PathBuf, String>,
    ) {
        match result {
            Ok(path) => {
                let label = sample_path_label(&path);
                self.waveform
                    .current
                    .flash_play_selection_if_current(&source_path, selection);
                self.ui.status.sample = format!("Copied {label} to clipboard");
                emit_gui_action(
                    "waveform.copy_playmarked_range",
                    Some("waveform"),
                    Some(&label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = format!("Copy failed: {error}");
                emit_gui_action(
                    "waveform.copy_playmarked_range",
                    Some("waveform"),
                    Some(&format!(
                        "{} {:.1}-{:.1}%",
                        sample_path_label(&source_path),
                        selection.start() * 100.0,
                        selection.end() * 100.0
                    )),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(in crate::native_app) fn external_drag_completed(
        &mut self,
        result: Result<ui::ExternalDragOutcome, String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        context.end_drag();
        self.library.folder_browser.clear_drag();
        self.clear_pending_internal_file_drag_paths();
        self.ui.status.sample = match result {
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
}
