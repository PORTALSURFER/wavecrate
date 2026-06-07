use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use radiant::runtime::{NativeFileDrop, NativeFileDropPhase};

use super::app_scope::{
    GuiMessage, NativeAppState, NativeFileDropHover, WAVEFORM_WIDGET_ID, emit_gui_action,
};

impl NativeAppState {
    pub(in crate::native_app) fn apply_native_file_drop(
        &mut self,
        drop: NativeFileDrop,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if self.folder_browser.drag_active() {
            self.apply_native_file_drop_during_browser_drag(drop, context);
            return;
        }
        if self.cancel_pending_internal_file_drag_drop(&drop, context) {
            return;
        }
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

    fn apply_native_file_drop_during_browser_drag(
        &mut self,
        drop: NativeFileDrop,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        self.native_file_drop_hover = None;
        match drop.phase {
            NativeFileDropPhase::Hover => {}
            NativeFileDropPhase::Cancel | NativeFileDropPhase::Drop => {
                self.folder_browser.clear_drag();
                self.clear_pending_internal_file_drag_paths();
                context.end_drag_session();
                self.sample_status = String::from("Drag cancelled");
            }
        }
    }

    fn cancel_pending_internal_file_drag_drop(
        &mut self,
        drop: &NativeFileDrop,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) -> bool {
        let should_cancel = match drop.phase {
            NativeFileDropPhase::Hover => drop
                .path
                .as_deref()
                .is_some_and(|path| self.is_pending_internal_file_drag_path(path)),
            NativeFileDropPhase::Cancel => !self.pending_internal_file_drag_paths.is_empty(),
            NativeFileDropPhase::Drop => drop
                .path
                .as_deref()
                .is_some_and(|path| self.is_pending_internal_file_drag_path(path)),
        };
        if !should_cancel {
            return false;
        }
        self.native_file_drop_hover = None;
        if !matches!(drop.phase, NativeFileDropPhase::Hover) {
            self.clear_pending_internal_file_drag_paths();
        }
        context.end_drag_session();
        self.sample_status = String::from("Drag cancelled");
        true
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
            self.sample_status = format!("Unsupported waveform drop: {}", file_name_or_path(&path));
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
        if self.drop_targets_selected_folder_file(&path) {
            self.sample_status = String::from("Drag cancelled");
            emit_gui_action(
                "waveform.external_file_drop",
                Some("waveform"),
                None,
                "cancelled",
                started_at,
                Some("drop target unchanged"),
            );
            return;
        }

        match self.copy_external_file_to_selected_folder(&path) {
            Ok(copied) => self.load_copied_external_file(copied, context, started_at),
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

    fn drop_targets_selected_folder_file(&self, source: &Path) -> bool {
        let Some(target_folder) = self.folder_browser.selected_folder_path() else {
            return false;
        };
        let Some(file_name) = source.file_name() else {
            return false;
        };
        paths_refer_to_same_file(source, &target_folder.join(file_name))
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
        let target = unique_copy_destination(&target_folder.join(file_name));
        fs::copy(source, &target).map_err(|err| {
            format!(
                "failed to copy {} to {}: {err}",
                source.display(),
                target.display()
            )
        })?;
        Ok(target)
    }

    fn load_copied_external_file(
        &mut self,
        copied: PathBuf,
        context: &mut ui::UpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
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

fn file_name_or_path(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
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

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
