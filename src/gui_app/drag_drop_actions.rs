use radiant::layout::Vector2;
use radiant::prelude as ui;
use radiant::runtime::{NativeFileDrop, NativeFileDropPhase};
use radiant::widgets::DragHandleMessage;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use wavecrate::external_clipboard;

use super::{
    DRAG_PREVIEW_HEIGHT, DRAG_PREVIEW_MAX_WIDTH, FolderBrowserMessage, GuiAppState, GuiMessage,
    NativeFileDropHover, WAVEFORM_WIDGET_ID, emit_gui_action,
};

impl GuiAppState {
    pub(super) fn drag_sample_file(
        &mut self,
        path: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        match drag {
            DragHandleMessage::Started { position } => {
                self.folder_browser.begin_file_drag(path, position);
                self.arm_browser_drag(context);
            }
            DragHandleMessage::Moved { position } => {
                self.folder_browser.update_drag_pointer(position);
            }
            DragHandleMessage::Ended { .. } => {
                self.folder_browser.clear_drag();
                context.end_drag();
                context.end_external_drag();
            }
        }
    }

    pub(super) fn drag_folder(
        &mut self,
        folder_id: String,
        drag: DragHandleMessage,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started = matches!(drag, DragHandleMessage::Started { .. });
        let ended = matches!(drag, DragHandleMessage::Ended { .. });
        if ended {
            if let Some(target_folder_id) = self.folder_browser.hovered_drop_target_folder_id() {
                self.drop_browser_drag_on_folder(target_folder_id, context);
            } else {
                self.folder_browser
                    .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
                context.end_drag();
                context.end_external_drag();
            }
            return;
        }
        self.folder_browser
            .apply_message(FolderBrowserMessage::DragFolder(folder_id, drag));
        if started {
            self.arm_browser_drag(context);
        }
    }

    fn arm_browser_drag(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        if let Some(preview) = self.folder_browser.drag_preview() {
            let width = folder_drag_preview_width(&preview.label);
            context.begin_drag(ui::DragRequest::new(
                ui::DragPreview::sized(preview.label, Vector2::new(width, DRAG_PREVIEW_HEIGHT)),
                preview.pointer,
            ));
        }
        let Some(request) = self.folder_browser.external_drag_request() else {
            return;
        };
        context.begin_external_drag(request, GuiMessage::ExternalDragCompleted);
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

    pub(super) fn apply_native_file_drop(
        &mut self,
        drop: NativeFileDrop,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let over_waveform = native_file_drop_targets_waveform(drop.target_widget);
        match drop.phase {
            NativeFileDropPhase::Hover => self.track_native_file_hover(drop.path, over_waveform),
            NativeFileDropPhase::Cancel => {
                self.native_file_drop_hover = None;
            }
            NativeFileDropPhase::Drop => {
                self.native_file_drop_hover = None;
                let Some(path) = drop.path else {
                    return;
                };
                if over_waveform {
                    self.drop_external_file_on_waveform(path, context);
                }
            }
        }
    }

    fn track_native_file_hover(&mut self, path: Option<PathBuf>, over_waveform: bool) {
        let Some(path) = path else {
            self.native_file_drop_hover = None;
            return;
        };
        self.native_file_drop_hover = over_waveform.then(|| NativeFileDropHover {
            supported: supported_waveform_drop_file(&path),
            path,
        });
    }

    fn drop_external_file_on_waveform(
        &mut self,
        path: PathBuf,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !supported_waveform_drop_file(&path) {
            self.sample_status = format!(
                "Unsupported waveform drop: {}",
                path.file_name()
                    .map(|name| name.to_string_lossy())
                    .unwrap_or_else(|| path.display().to_string().into())
            );
            emit_gui_action(
                "waveform.external_file_drop",
                Some("waveform"),
                None,
                "unsupported",
                started_at,
                Some("unsupported file type"),
            );
            return;
        }
        match self.copy_external_file_to_selected_folder(&path) {
            Ok(copied) => {
                let copied_id = copied.display().to_string();
                self.folder_browser.refresh_file_path(&copied);
                self.folder_browser.select_file(copied_id.clone());
                self.load_sample(copied_id, context);
                emit_gui_action(
                    "waveform.external_file_drop",
                    Some("waveform"),
                    None,
                    "copied",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("External drop failed: {error}");
                emit_gui_action(
                    "waveform.external_file_drop",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn copy_external_file_to_selected_folder(&mut self, source: &Path) -> Result<PathBuf, String> {
        if !source.is_file() {
            return Err(format!("not a file: {}", source.display()));
        }
        let target_folder = self
            .folder_browser
            .selected_folder_path()
            .ok_or_else(|| String::from("no selected folder"))?;
        fs::create_dir_all(&target_folder).map_err(|err| {
            format!(
                "failed to create target folder {}: {err}",
                target_folder.display()
            )
        })?;
        let file_name = source
            .file_name()
            .ok_or_else(|| String::from("dropped file has no file name"))?;
        let first_candidate = target_folder.join(file_name);
        let target = unique_copy_destination(&first_candidate);
        fs::copy(source, &target).map_err(|err| {
            format!(
                "failed to copy {} to {}: {err}",
                source.display(),
                target.display()
            )
        })?;
        Ok(target)
    }

    pub(super) fn external_drag_completed(
        &mut self,
        result: Result<ui::ExternalDragOutcome, String>,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        context.end_drag();
        self.folder_browser.clear_drag();
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
        context.end_drag();
        context.end_external_drag();
        match self.folder_browser.drop_drag_on_folder(&folder_id) {
            Ok(result) => {
                for (old_path, new_path) in &result.moved_paths {
                    self.waveform.rewrite_path_prefix(old_path, new_path);
                }
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
}

fn folder_drag_preview_width(label: &str) -> f32 {
    (label.chars().count() as f32 * 7.0 + 28.0).clamp(96.0, DRAG_PREVIEW_MAX_WIDTH)
}

fn supported_waveform_drop_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn native_file_drop_targets_waveform(target_widget: Option<u64>) -> bool {
    target_widget.is_none() || target_widget == Some(WAVEFORM_WIDGET_ID)
}

fn unique_copy_destination(first_candidate: &Path) -> PathBuf {
    if !first_candidate.exists() {
        return first_candidate.to_path_buf();
    }
    let parent = first_candidate.parent().unwrap_or_else(|| Path::new(""));
    let stem = first_candidate
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = first_candidate
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    for count in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_copy{count:03}.{extension}"),
            None => format!("{stem}_copy{count:03}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded copy suffix search should find a destination")
}
