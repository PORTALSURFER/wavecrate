use super::ops;
use super::*;
use crate::app::state::FocusContext;
use std::path::{Path, PathBuf};

impl AppController {
    pub(crate) fn bind_folder_hotkey(&mut self, folder: &Path, hotkey: Option<u8>) {
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        let slot = match ops::normalize_folder_hotkey(hotkey) {
            Ok(slot) => slot,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return;
            }
        };
        if !folder.as_os_str().is_empty() && !source.root.join(folder).is_dir() {
            self.set_status(
                format!("Folder missing: {}", folder.display()),
                StatusTone::Error,
            );
            return;
        }
        let (snapshot, name) = match self.apply_folder_hotkey_binding(folder, slot) {
            Ok(state) => state,
            Err(err) => {
                self.set_status(err, StatusTone::Error);
                return;
            }
        };
        self.build_folder_rows(&snapshot);
        match slot {
            Some(slot) => {
                self.set_status(format!("Bound hotkey {slot} to '{name}'"), StatusTone::Info)
            }
            None => self.set_status(format!("Cleared hotkey for '{name}'"), StatusTone::Info),
        }
    }

    pub(crate) fn apply_folder_hotkey(&mut self, hotkey: u8, focus: FocusContext) -> bool {
        let Some(target) = self.resolve_folder_hotkey_target(hotkey, focus) else {
            return false;
        };
        match target {
            FolderHotkeyTarget::Missing => true,
            FolderHotkeyTarget::Ready { source, folder } => {
                self.run_folder_hotkey_move(&source, &folder);
                true
            }
        }
    }

    fn apply_folder_hotkey_binding(
        &mut self,
        folder: &Path,
        slot: Option<u8>,
    ) -> Result<(FolderBrowserModel, String), String> {
        let name = if folder.as_os_str().is_empty() {
            ".".to_string()
        } else {
            folder.to_string_lossy().into_owned()
        };
        let Some(model) = self.current_folder_model_mut() else {
            return Err("Select a source first".into());
        };
        model
            .hotkeys
            .retain(|key, path| *key != slot.unwrap_or(255) && path != folder);
        if let Some(slot) = slot {
            model.hotkeys.insert(slot, folder.to_path_buf());
        }
        Ok((model.clone(), name))
    }

    fn folder_for_hotkey(&self, hotkey: u8) -> Option<PathBuf> {
        self.current_folder_model()
            .and_then(|model| model.hotkeys.get(&hotkey).cloned())
    }

    fn resolve_folder_hotkey_target(
        &mut self,
        hotkey: u8,
        focus: FocusContext,
    ) -> Option<FolderHotkeyTarget> {
        if !matches!(focus, FocusContext::SampleBrowser) {
            return None;
        }
        let source = self.current_source()?;
        let folder = self.folder_for_hotkey(hotkey)?;
        if !folder.as_os_str().is_empty() && !source.root.join(&folder).is_dir() {
            self.set_status(
                format!("Folder missing: {}", folder.display()),
                StatusTone::Error,
            );
            return Some(FolderHotkeyTarget::Missing);
        }
        Some(FolderHotkeyTarget::Ready { source, folder })
    }

    fn next_focus_path_after_folder_move(&mut self, rows: &[usize]) -> Option<PathBuf> {
        if rows.is_empty() || self.ui.browser.viewport.visible.len() == 0 {
            return None;
        }
        let mut sorted = rows.to_vec();
        sorted.sort_unstable();
        let highest = sorted.last().copied()?;
        let first = sorted.first().copied().unwrap_or(highest);
        let after = highest
            .checked_add(1)
            .and_then(|idx| self.ui.browser.viewport.visible.get(idx))
            .and_then(|entry_idx| self.wav_entry(entry_idx))
            .map(|entry| entry.relative_path.clone());
        after.or_else(|| {
            first
                .checked_sub(1)
                .and_then(|idx| self.ui.browser.viewport.visible.get(idx))
                .and_then(|entry_idx| self.wav_entry(entry_idx))
                .map(|entry| entry.relative_path.clone())
        })
    }

    fn apply_folder_move_focus(&mut self, next_focus: Option<PathBuf>) {
        let Some(path) = next_focus else {
            return;
        };
        if let Some(row) = self.visible_row_for_path(&path) {
            self.focus_browser_row_only(row);
        } else if self.wav_index_for_path(&path).is_some() {
            self.select_wav_by_path_with_rebuild(&path, true);
        }
    }

    fn run_folder_hotkey_move(&mut self, source: &SampleSource, folder: &Path) {
        let rows = self.browser_selection_rows_for_drag_samples();
        if rows.is_empty() {
            self.set_status("Select samples to move to a folder", StatusTone::Info);
            return;
        }
        let samples = self.drag_samples_for_browser_rows(source, &rows);
        if samples.is_empty() {
            self.set_status("No samples available for folder move", StatusTone::Warning);
            return;
        }
        let next_focus = self.next_focus_path_after_folder_move(&rows);
        self.move_samples_to_folder(samples, folder.to_path_buf());
        self.clear_browser_selection();
        self.apply_folder_move_focus(next_focus);
    }
}

enum FolderHotkeyTarget {
    Missing,
    Ready {
        source: SampleSource,
        folder: PathBuf,
    },
}
