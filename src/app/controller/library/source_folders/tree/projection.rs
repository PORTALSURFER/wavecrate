//! Folder-tree projection, local row patching, and available-folder collection.

use super::snapshot::{FolderTreeSnapshot, build_folder_row_view};
use super::*;
use crate::app::state::FolderRowView;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

/// Minimal projected folder-browser fields produced by sync or async row projection.
#[derive(Clone, Debug, Default)]
pub(crate) struct FolderProjectionView {
    /// Render rows for the folder tree.
    pub(crate) rows: Vec<FolderRowView>,
    /// Focused row index inside `rows`, when any.
    pub(crate) focused: Option<usize>,
    /// Row index that should scroll into view, when any.
    pub(crate) scroll_to: Option<usize>,
    /// Active search query.
    pub(crate) search_query: String,
    /// Whether empty disk folders should be shown.
    pub(crate) show_all_folders: bool,
    /// Whether descendant files are included by folder filtering.
    pub(crate) flattened_view: bool,
}

impl AppController {
    /// Project folder rows synchronously into the active pane UI.
    pub(crate) fn build_folder_rows(&mut self, model: &FolderBrowserModel) {
        let source_id = self.selection_state.ctx.selected_source.clone();
        let snapshot = source_id
            .map(|source_id| {
                self.ui_cache
                    .folders
                    .snapshots
                    .get(&folder_browser_cache_key(
                        self.active_folder_pane(),
                        source_id,
                    ))
                    .cloned()
            })
            .flatten()
            .unwrap_or_else(|| FolderTreeSnapshot::from_available(&model.available));
        let view = project_folder_browser_view(
            model,
            &snapshot,
            self.selection_state.ctx.selected_source.is_some(),
        );
        self.apply_folder_projection_view(self.active_folder_pane(), view);
    }

    /// Apply one folder projection view into the targeted pane UI.
    pub(crate) fn apply_folder_projection_view(
        &mut self,
        pane: FolderPaneId,
        view: FolderProjectionView,
    ) {
        let is_active = self.ui.sources.active_folder_pane == pane;
        let ui = if is_active {
            &mut self.ui.sources.folders
        } else {
            &mut self.ui.sources.folder_pane_mut(pane).browser
        };
        ui.rows = view.rows;
        ui.focused = view.focused;
        ui.scroll_to = view.scroll_to;
        ui.search_query = view.search_query;
        ui.show_all_folders = view.show_all_folders;
        ui.flattened_view = view.flattened_view;
        if is_active {
            self.sync_active_folder_ui_to_pane();
            self.mark_folder_search_projection_revision_dirty();
        }
    }

    /// Patch the current pane UI locally when row structure is unchanged.
    pub(crate) fn patch_current_folder_ui_locally(
        &mut self,
        pane: FolderPaneId,
        model: &FolderBrowserModel,
        update_expanded: bool,
    ) -> bool {
        let hotkey_lookup = model
            .hotkeys
            .iter()
            .map(|(slot, path)| (path.clone(), *slot))
            .collect::<BTreeMap<_, _>>();
        let ui = if self.ui.sources.active_folder_pane == pane {
            &mut self.ui.sources.folders
        } else {
            &mut self.ui.sources.folder_pane_mut(pane).browser
        };
        let focused = model.focused.as_ref().and_then(|path| {
            ui.rows.iter().position(|row| {
                if row.is_root {
                    path.as_os_str().is_empty()
                } else {
                    &row.path == path
                }
            })
        });
        if model.focused.is_some() && focused.is_none() {
            return false;
        }
        for row in &mut ui.rows {
            if !row.is_root && !model.available.contains(&row.path) {
                return false;
            }
            row.selected = model.selected.contains(&row.path);
            row.negated = model.negated.contains(&row.path);
            row.hotkey = hotkey_lookup.get(&row.path).copied();
            if row.is_root {
                row.file_scope_mode = Some(model.file_scope_mode);
            } else if update_expanded {
                row.expanded = model.expanded.contains(&row.path);
            }
        }
        ui.focused = focused;
        ui.scroll_to = focused;
        ui.last_focused_path = model.focused.clone();
        ui.search_query = model.search_query.clone();
        ui.show_all_folders = model.show_all_folders;
        ui.flattened_view =
            model.file_scope_mode == crate::app::state::FolderFileScopeMode::AllDescendants;
        if self.ui.sources.active_folder_pane == pane {
            self.sync_active_folder_ui_to_pane();
        }
        true
    }

    pub(super) fn collect_folders(
        &mut self,
        source_root: &Path,
        include_disk: bool,
    ) -> BTreeSet<PathBuf> {
        let mut candidates = BTreeSet::new();
        for index in 0..self.wav_entries_len() {
            let Some(entry) = self.wav_entry(index) else {
                continue;
            };
            let mut current = entry.relative_path.parent();
            while let Some(path) = current {
                if !path.as_os_str().is_empty() {
                    candidates.insert(path.to_path_buf());
                }
                current = path.parent();
            }
        }

        let mut folders = BTreeSet::new();
        for path in candidates {
            if source_root.join(&path).is_dir() {
                folders.insert(path);
            }
        }
        if include_disk {
            let cancel = AtomicBool::new(false);
            folders.extend(scan_disk_folders(source_root, &cancel));
        }
        folders
    }
}

/// Project folder rows from one retained model and immutable snapshot.
pub(crate) fn project_folder_browser_view(
    model: &FolderBrowserModel,
    snapshot: &FolderTreeSnapshot,
    has_source: bool,
) -> FolderProjectionView {
    let hotkey_lookup: BTreeMap<PathBuf, u8> = model
        .hotkeys
        .iter()
        .map(|(slot, path)| (path.clone(), *slot))
        .collect();
    let searching = !model.search_query.trim().is_empty();
    let rows = if searching {
        project_searched_rows(model, snapshot, &hotkey_lookup)
    } else {
        project_visible_rows(model, snapshot, &hotkey_lookup, has_source)
    };
    let focused = model
        .focused
        .as_ref()
        .and_then(|path| rows.iter().position(|row| &row.path == path));
    FolderProjectionView {
        rows,
        focused,
        scroll_to: focused,
        search_query: model.search_query.clone(),
        show_all_folders: model.show_all_folders,
        flattened_view: model.file_scope_mode
            == crate::app::state::FolderFileScopeMode::AllDescendants,
    }
}

fn project_visible_rows(
    model: &FolderBrowserModel,
    snapshot: &FolderTreeSnapshot,
    hotkeys: &BTreeMap<PathBuf, u8>,
    has_source: bool,
) -> Vec<FolderRowView> {
    let mut rows = Vec::new();
    if has_source {
        rows.push(FolderRowView {
            path: PathBuf::new(),
            name: String::from("Root"),
            depth: 0,
            has_children: snapshot.root_has_children(),
            expanded: true,
            selected: model.selected.contains(Path::new("")),
            negated: model.negated.contains(Path::new("")),
            hotkey: hotkeys.get(Path::new("")).copied(),
            is_root: true,
            file_scope_mode: Some(model.file_scope_mode),
        });
    }
    flatten_visible_rows(Path::new(""), snapshot, model, hotkeys, &mut rows);
    rows
}

fn flatten_visible_rows(
    parent: &Path,
    snapshot: &FolderTreeSnapshot,
    model: &FolderBrowserModel,
    hotkeys: &BTreeMap<PathBuf, u8>,
    rows: &mut Vec<FolderRowView>,
) {
    let Some(children) = snapshot.children.get(parent) else {
        return;
    };
    for child in children {
        let Some(static_row) = snapshot.row(child) else {
            continue;
        };
        rows.push(build_folder_row_view(
            static_row,
            model.selected.contains(child),
            model.negated.contains(child),
            model.expanded.contains(child),
            hotkeys.get(child).copied(),
        ));
        if static_row.has_children && model.expanded.contains(child) {
            flatten_visible_rows(child, snapshot, model, hotkeys, rows);
        }
    }
}

fn project_searched_rows(
    model: &FolderBrowserModel,
    snapshot: &FolderTreeSnapshot,
    hotkeys: &BTreeMap<PathBuf, u8>,
) -> Vec<FolderRowView> {
    let matcher = SkimMatcherV2::default();
    let mut scored = Vec::new();
    for path in &snapshot.flattened_paths {
        let Some(static_row) = snapshot.row(path) else {
            continue;
        };
        let label = path.to_string_lossy();
        if let Some(score) = matcher.fuzzy_match(label.as_ref(), &model.search_query) {
            scored.push((
                build_folder_row_view(
                    static_row,
                    model.selected.contains(path),
                    model.negated.contains(path),
                    model.expanded.contains(path),
                    hotkeys.get(path).copied(),
                ),
                score,
            ));
        }
    }
    scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.path.cmp(&b.0.path)));
    scored.into_iter().map(|(row, _)| row).collect()
}
