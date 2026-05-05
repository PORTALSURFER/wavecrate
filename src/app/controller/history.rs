//! Typed history helpers for meaningful controller state transitions.
//!
//! This module keeps undoable UI snapshots close to the controller so
//! navigation and selection flows can reuse one consistent history model
//! instead of pushing ad hoc closure entries throughout feature code.

mod catalog;
mod pending;
mod transactions;

use super::*;
use crate::app::state::{FolderFileScopeMode, FolderPaneId, WaveformView};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
pub(crate) use self::catalog::catalog_history_handler_supported;
pub(crate) use self::pending::{
    PendingHistoryTransaction, PendingHistoryTransactionKey, PendingSampleCreationTransaction,
    PendingSampleOverwriteTransaction,
};

/// Reversible folder-browser state owned by one selected source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FolderHistorySnapshot {
    /// Currently selected folder paths.
    pub selected: BTreeSet<PathBuf>,
    /// Folder paths excluded from filters.
    pub negated: BTreeSet<PathBuf>,
    /// Expanded folder tree paths.
    pub expanded: BTreeSet<PathBuf>,
    /// Focused folder path.
    pub focused: Option<PathBuf>,
    /// Shift-selection anchor path.
    pub selection_anchor: Option<PathBuf>,
    /// Manual folders retained even when they have no samples yet.
    pub manual_folders: BTreeSet<PathBuf>,
    /// Assigned hotkey slots for folder jumps.
    pub hotkeys: BTreeMap<u8, PathBuf>,
    /// Whether empty folders discovered on disk stay visible in the tree.
    pub show_all_folders: bool,
    /// Folder file-scope mode used by browser filtering.
    pub file_scope_mode: FolderFileScopeMode,
}

/// Reversible subset of controller state that represents meaningful UI context.
/// This intentionally excludes transient playback helpers such as compare-anchor state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MeaningfulUiSnapshot {
    /// Currently selected source id.
    pub selected_source: Option<SourceId>,
    /// Upper pane source assignment.
    pub upper_folder_pane_source: Option<SourceId>,
    /// Lower pane source assignment.
    pub lower_folder_pane_source: Option<SourceId>,
    /// Pane currently driving the browser/waveform.
    pub active_folder_pane: FolderPaneId,
    /// Last selected browsable source id.
    pub last_selected_browsable_source: Option<SourceId>,
    /// Browser-selected relative paths.
    pub browser_selected_paths: Vec<PathBuf>,
    /// Browser range-selection anchor row.
    pub browser_selection_anchor_visible: Option<usize>,
    /// Last focused browser entry index.
    pub browser_last_focused_index: Option<usize>,
    /// Last focused browser relative path.
    pub browser_last_focused_path: Option<PathBuf>,
    /// Whether browser autoscroll was enabled.
    pub browser_autoscroll: bool,
    /// Selected waveform/browser path.
    pub selected_wav: Option<PathBuf>,
    /// Loaded waveform path, if any.
    pub loaded_wav: Option<PathBuf>,
    /// Focused folder row index in the rendered tree.
    pub folder_ui_focused: Option<usize>,
    /// Previously focused folder path.
    pub folder_ui_last_focused_path: Option<PathBuf>,
    /// Reversible folder state for the selected source.
    pub folder_state: Option<FolderHistorySnapshot>,
    /// Active playback selection.
    pub waveform_selection: Option<SelectionRange>,
    /// Active edit selection.
    pub waveform_edit_selection: Option<SelectionRange>,
    /// Waveform viewport window.
    pub waveform_view: WaveformView,
    /// Waveform cursor position.
    pub waveform_cursor: Option<f32>,
    /// Loop enabled state visible in the waveform chrome.
    pub waveform_loop_enabled: bool,
}

impl AppController {
    /// Return whether history replay is currently restoring controller state.
    pub(crate) fn history_restoring(&self) -> bool {
        self.history.restoring
    }

    /// Run one controller mutation while suppressing new history capture.
    pub(crate) fn run_history_restore(&mut self, mut restore: impl FnMut(&mut Self)) {
        let was_restoring = self.history.restoring;
        self.history.restoring = true;
        restore(self);
        self.history.restoring = was_restoring;
    }

    /// Run one meaningful UI mutation and push a snapshot-based undo entry when it changes state.
    pub(crate) fn record_meaningful_ui_transaction<R>(
        &mut self,
        label: impl Into<String>,
        action: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if self.history_restoring() {
            return action(self);
        }
        let label = label.into();
        let before = self.capture_meaningful_ui_snapshot();
        let result = action(self);
        self.push_meaningful_ui_undo_if_changed(label, before);
        result
    }

    /// Capture the meaningful UI context that should be restored by undo/redo.
    pub(crate) fn capture_meaningful_ui_snapshot(&self) -> MeaningfulUiSnapshot {
        let folder_cache_key = self
            .selection_state
            .ctx
            .selected_source
            .as_ref()
            .map(
                |source_id| crate::app::controller::state::cache::FolderBrowserCacheKey {
                    pane: self.ui.sources.active_folder_pane,
                    source_id: source_id.clone(),
                },
            );
        let folder_state = folder_cache_key
            .as_ref()
            .and_then(|key| self.ui_cache.folders.models.get(key))
            .map(|model| FolderHistorySnapshot {
                selected: model.selected.clone(),
                negated: model.negated.clone(),
                expanded: model.expanded.clone(),
                focused: model.focused.clone(),
                selection_anchor: model.selection_anchor.clone(),
                manual_folders: model.manual_folders.clone(),
                hotkeys: model.hotkeys.clone(),
                show_all_folders: model.show_all_folders,
                file_scope_mode: model.file_scope_mode,
            });

        MeaningfulUiSnapshot {
            selected_source: self.selection_state.ctx.selected_source.clone(),
            upper_folder_pane_source: self.folder_pane_source(FolderPaneId::Upper),
            lower_folder_pane_source: self.folder_pane_source(FolderPaneId::Lower),
            active_folder_pane: self.ui.sources.active_folder_pane,
            last_selected_browsable_source: self
                .selection_state
                .ctx
                .last_selected_browsable_source
                .clone(),
            browser_selected_paths: self.ui.browser.selection.selected_paths.clone(),
            browser_selection_anchor_visible: self.ui.browser.selection.selection_anchor_visible,
            browser_last_focused_index: self.ui.browser.selection.last_focused_index,
            browser_last_focused_path: self.ui.browser.selection.last_focused_path.clone(),
            browser_autoscroll: self.ui.browser.selection.autoscroll,
            selected_wav: self.sample_view.wav.selected_wav.clone(),
            loaded_wav: self.sample_view.wav.loaded_wav.clone(),
            folder_ui_focused: self.ui.sources.folders.focused,
            folder_ui_last_focused_path: self.ui.sources.folders.last_focused_path.clone(),
            folder_state,
            waveform_selection: self
                .selection_state
                .range
                .range()
                .or(self.ui.waveform.selection),
            waveform_edit_selection: self
                .selection_state
                .edit_range
                .range()
                .or(self.ui.waveform.edit_selection),
            waveform_view: self.ui.waveform.view,
            waveform_cursor: self.ui.waveform.cursor,
            waveform_loop_enabled: self.ui.waveform.loop_enabled,
        }
    }

    /// Restore a previously captured meaningful UI context without recording history.
    pub(crate) fn restore_meaningful_ui_snapshot(&mut self, snapshot: &MeaningfulUiSnapshot) {
        let snapshot = snapshot.clone();
        self.run_history_restore(|controller| {
            controller
                .selection_state
                .ctx
                .last_selected_browsable_source = snapshot.last_selected_browsable_source.clone();
            controller.sync_active_folder_ui_to_pane();
            controller.ui.sources.folder_panes.upper.source_id =
                snapshot.upper_folder_pane_source.clone();
            controller.ui.sources.folder_panes.lower.source_id =
                snapshot.lower_folder_pane_source.clone();
            controller.ui.sources.active_folder_pane = snapshot.active_folder_pane;
            controller.selection_state.ctx.selected_source = snapshot.selected_source.clone();
            controller.load_active_folder_ui_from_pane();

            if let Some(source_id) = snapshot.selected_source.clone() {
                let model = controller
                    .ui_cache
                    .folders
                    .models
                    .entry(
                        crate::app::controller::state::cache::FolderBrowserCacheKey {
                            pane: controller.ui.sources.active_folder_pane,
                            source_id,
                        },
                    )
                    .or_default();
                if let Some(folder_state) = snapshot.folder_state.as_ref() {
                    model.selected = folder_state.selected.clone();
                    model.negated = folder_state.negated.clone();
                    model.expanded = folder_state.expanded.clone();
                    model.focused = folder_state.focused.clone();
                    model.selection_anchor = folder_state.selection_anchor.clone();
                    model.manual_folders = folder_state.manual_folders.clone();
                    model.hotkeys = folder_state.hotkeys.clone();
                    model.show_all_folders = folder_state.show_all_folders;
                    model.file_scope_mode = folder_state.file_scope_mode;
                }
            }

            controller.refresh_sources_ui();
            controller.refresh_folder_browser();
            controller.ui.sources.folders.focused = snapshot.folder_ui_focused;
            controller.ui.sources.folders.scroll_to = snapshot.folder_ui_focused;
            controller.ui.sources.folders.last_focused_path =
                snapshot.folder_ui_last_focused_path.clone();

            controller.sample_view.wav.loaded_audio = None;
            controller.sample_view.wav.selected_wav = None;
            controller.sample_view.wav.loaded_wav = None;
            controller.set_ui_loaded_wav(None);
            controller.clear_focused_similarity_highlight();

            if let Some(source) = controller.current_source() {
                if let Some(path) = snapshot.selected_wav.clone() {
                    controller.selection_state.suppress_autoplay_once = true;
                    controller.select_wav_by_path_with_rebuild(&path, true);
                } else {
                    controller.rebuild_browser_lists();
                }

                if let Some(path) = snapshot.loaded_wav.clone() {
                    let _ = controller.queue_audio_load_for(
                        &source,
                        &path,
                        AudioLoadIntent::Selection,
                        None,
                    );
                }
            } else {
                controller.rebuild_browser_lists();
            }

            controller.set_browser_selected_paths(snapshot.browser_selected_paths.clone());
            controller.ui.browser.selection.selection_anchor_visible =
                snapshot.browser_selection_anchor_visible;
            controller.ui.browser.selection.last_focused_index =
                snapshot.browser_last_focused_index;
            controller.ui.browser.selection.last_focused_path =
                snapshot.browser_last_focused_path.clone();
            controller.ui.browser.selection.autoscroll = snapshot.browser_autoscroll;
            controller.refresh_browser_selection_markers();

            controller
                .selection_state
                .range
                .set_range(snapshot.waveform_selection);
            controller.apply_selection(snapshot.waveform_selection);
            controller
                .selection_state
                .edit_range
                .set_range(snapshot.waveform_edit_selection);
            controller.selection_state.edit_fade_drag = None;
            controller.apply_edit_selection(snapshot.waveform_edit_selection);
            controller.ui.waveform.view = snapshot.waveform_view.clamp();
            controller.ui.waveform.cursor = snapshot.waveform_cursor;
            controller.ui.waveform.loop_enabled = snapshot.waveform_loop_enabled;
        });
    }

    /// Push one undo entry that restores meaningful UI state before and after an action.
    pub(crate) fn push_meaningful_ui_undo(
        &mut self,
        label: impl Into<String>,
        before: MeaningfulUiSnapshot,
        after: MeaningfulUiSnapshot,
    ) {
        if self.history_restoring() || before == after {
            return;
        }
        let label = label.into();
        self.push_undo_entry(undo::UndoEntry::<AppController>::new(
            label,
            move |controller| {
                controller.restore_meaningful_ui_snapshot(&before);
                Ok(undo::UndoExecution::Applied)
            },
            move |controller| {
                controller.restore_meaningful_ui_snapshot(&after);
                Ok(undo::UndoExecution::Applied)
            },
        ));
    }

    /// Capture the post-action snapshot and push an undo entry when state changed.
    pub(crate) fn push_meaningful_ui_undo_if_changed(
        &mut self,
        label: impl Into<String>,
        before: MeaningfulUiSnapshot,
    ) {
        if self.history_restoring() {
            return;
        }
        let label = label.into();
        let after = self.capture_meaningful_ui_snapshot();
        self.push_meaningful_ui_undo(label, before, after);
    }
}
