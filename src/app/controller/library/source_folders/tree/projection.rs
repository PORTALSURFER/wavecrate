//! Folder-tree projection and fuzzy-filter helpers.

use super::*;
use crate::app::state::FolderRowView;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

impl AppController {
    pub(crate) fn build_folder_rows(&mut self, model: &FolderBrowserModel) {
        self.set_ui_folder_search_query(model.search_query.clone());
        let hotkey_lookup: BTreeMap<PathBuf, u8> = model
            .hotkeys
            .iter()
            .map(|(slot, path)| (path.clone(), *slot))
            .collect();
        let tree = self.build_folder_tree(&model.available);
        let searching = !model.search_query.trim().is_empty();
        let has_source = self.selection_state.ctx.selected_source.is_some();
        let mut folder_rows = Vec::new();
        let expanded = if searching {
            model.available.clone()
        } else {
            model.expanded.clone()
        };
        Self::flatten_folder_tree(
            Path::new(""),
            usize::from(has_source),
            &tree,
            model,
            &expanded,
            &hotkey_lookup,
            &mut folder_rows,
        );
        if searching {
            folder_rows = self.filter_folder_rows(folder_rows, &model.search_query);
        }
        let mut rows = Vec::new();
        if has_source && !searching {
            let has_children = !folder_rows.is_empty();
            let hotkey = hotkey_lookup.get(Path::new("")).copied();
            rows.push(FolderRowView {
                path: PathBuf::new(),
                name: String::from("Root"),
                depth: 0,
                has_children,
                expanded: true,
                selected: model.selected.contains(Path::new("")),
                negated: model.negated.contains(Path::new("")),
                hotkey,
                is_root: true,
                root_filter_mode: Some(model.root_filter_mode),
            });
        }
        rows.extend(folder_rows);
        let focused = model
            .focused
            .as_ref()
            .and_then(|path| rows.iter().position(|row| &row.path == path));
        self.ui.sources.folders.rows = rows;
        self.ui.sources.folders.focused = focused;
        self.ui.sources.folders.scroll_to = focused;
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

    fn build_folder_tree(&self, available: &BTreeSet<PathBuf>) -> BTreeMap<PathBuf, Vec<PathBuf>> {
        let mut tree: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
        for path in available {
            let parent = path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(PathBuf::new);
            tree.entry(parent).or_default().push(path.clone());
        }
        for children in tree.values_mut() {
            children.sort();
        }
        tree
    }

    fn filter_folder_rows(&self, rows: Vec<FolderRowView>, query: &str) -> Vec<FolderRowView> {
        let matcher = SkimMatcherV2::default();
        let mut scored = Vec::new();
        for row in rows {
            let label = row.path.to_string_lossy();
            if let Some(score) = matcher.fuzzy_match(label.as_ref(), query) {
                scored.push((row, score));
            }
        }
        scored.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.path.cmp(&b.0.path)));
        scored.into_iter().map(|(row, _)| row).collect()
    }

    fn flatten_folder_tree(
        parent: &Path,
        depth: usize,
        tree: &BTreeMap<PathBuf, Vec<PathBuf>>,
        model: &FolderBrowserModel,
        expanded: &BTreeSet<PathBuf>,
        hotkeys: &BTreeMap<PathBuf, u8>,
        rows: &mut Vec<FolderRowView>,
    ) {
        let Some(children) = tree.get(parent) else {
            return;
        };
        for child in children {
            let has_children = tree.contains_key(child);
            let is_expanded = expanded.contains(child);
            let selected = model.selected.contains(child);
            let negated = model.negated.contains(child);
            let hotkey = hotkeys.get(child).copied();
            let name = child
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
                .unwrap_or_else(|| child.display().to_string());
            let row = FolderRowView {
                path: child.clone(),
                name,
                depth,
                has_children,
                expanded: is_expanded,
                selected,
                negated,
                hotkey,
                is_root: false,
                root_filter_mode: None,
            };
            rows.push(row);
            if has_children && is_expanded {
                Self::flatten_folder_tree(child, depth + 1, tree, model, expanded, hotkeys, rows);
            }
        }
    }
}
