use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{FileEntry, file_entry, folder_label, path_id, rewrite_path_id};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::native_app) struct FolderEntry {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) children: Vec<FolderEntry>,
    pub(super) files: Vec<FileEntry>,
}

impl FolderEntry {
    pub(super) fn find(&self, id: &str) -> Option<&FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }

    pub(super) fn find_mut(&mut self, id: &str) -> Option<&mut FolderEntry> {
        if self.id == id {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(id))
    }

    pub(super) fn find_file(&self, id: &str) -> Option<&FileEntry> {
        self.files
            .iter()
            .find(|file| file.id == id)
            .or_else(|| self.children.iter().find_map(|child| child.find_file(id)))
    }

    pub(super) fn all_files(&self) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        self.collect_files(&mut files);
        files
    }

    fn collect_files<'a>(&'a self, files: &mut Vec<&'a FileEntry>) {
        files.extend(self.files.iter());
        for child in &self.children {
            child.collect_files(files);
        }
    }

    pub(super) fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub(super) fn contains_audio(&self) -> bool {
        self.files.iter().any(FileEntry::is_audio)
            || self.children.iter().any(FolderEntry::contains_audio)
    }

    pub(super) fn collect_folder_ids(&self, ids: &mut HashSet<String>) {
        ids.insert(self.id.clone());
        for child in &self.children {
            child.collect_folder_ids(ids);
        }
    }

    pub(super) fn replace_direct_entries(
        &mut self,
        child_paths: Vec<PathBuf>,
        files: Vec<FileEntry>,
    ) -> bool {
        let previous_children = std::mem::take(&mut self.children);
        let next_children = child_paths
            .into_iter()
            .map(|path| {
                let id = path_id(&path);
                previous_children
                    .iter()
                    .find(|child| child.id == id)
                    .cloned()
                    .unwrap_or_else(|| FolderEntry {
                        id,
                        name: folder_label(&path),
                        children: Vec::new(),
                        files: Vec::new(),
                    })
            })
            .collect::<Vec<_>>();

        let next_files = files
            .into_iter()
            .map(|mut file| {
                if let Some(previous) = self.files.iter().find(|previous| previous.id == file.id) {
                    file.rating = previous.rating;
                    file.rating_locked = previous.rating_locked;
                    file.collection = previous.collection;
                    file.collections = previous.collections.clone();
                }
                file
            })
            .collect::<Vec<_>>();

        let changed = self.children != next_children || self.files != next_files;
        self.children = next_children;
        self.files = next_files;
        changed
    }

    pub(super) fn replace_folder_structure(&mut self, fresh: FolderEntry) -> bool {
        let previous_name = std::mem::replace(&mut self.name, fresh.name);
        let previous_children = std::mem::take(&mut self.children);
        let next_children = fresh
            .children
            .into_iter()
            .map(|fresh_child| {
                if let Some(mut previous_child) = previous_children
                    .iter()
                    .find(|child| child.id == fresh_child.id)
                    .cloned()
                {
                    previous_child.replace_folder_structure(fresh_child);
                    previous_child
                } else {
                    fresh_child
                }
            })
            .collect::<Vec<_>>();
        let changed = previous_name != self.name || previous_children != next_children;
        self.children = next_children;
        changed
    }

    pub(super) fn rewrite_path_prefix(&mut self, old_path: &Path, new_path: &Path) {
        self.id = rewrite_path_id(&self.id, old_path, new_path);
        if Path::new(&self.id) == new_path {
            self.name = folder_label(new_path);
        }
        for child in &mut self.children {
            child.rewrite_path_prefix(old_path, new_path);
        }
        for file in &mut self.files {
            file.id = rewrite_path_id(&file.id, old_path, new_path);
        }
    }

    pub(super) fn rewrite_file_path(&mut self, old_path: &Path, new_path: &Path) -> bool {
        let old_id = path_id(old_path);
        for file in &mut self.files {
            if file.id == old_id {
                let previous = file.clone();
                let mut renamed = file_entry(&PathBuf::from(new_path));
                renamed.rating = previous.rating;
                renamed.rating_locked = previous.rating_locked;
                renamed.collection = previous.collection;
                renamed.collections = previous.collections;
                *file = renamed;
                self.files.sort_by(|a, b| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                });
                return true;
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.rewrite_file_path(old_path, new_path))
    }

    pub(super) fn remove_child_by_id(&mut self, target_id: &str) -> bool {
        if let Some(index) = self.children.iter().position(|child| child.id == target_id) {
            self.children.remove(index);
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.remove_child_by_id(target_id))
    }

    pub(super) fn take_child_by_id(&mut self, target_id: &str) -> Option<FolderEntry> {
        if let Some(index) = self.children.iter().position(|child| child.id == target_id) {
            return Some(self.children.remove(index));
        }
        self.children
            .iter_mut()
            .find_map(|child| child.take_child_by_id(target_id))
    }

    pub(super) fn take_file_by_id(&mut self, target_id: &str) -> Option<FileEntry> {
        if let Some(index) = self.files.iter().position(|file| file.id == target_id) {
            return Some(self.files.remove(index));
        }
        self.children
            .iter_mut()
            .find_map(|child| child.take_file_by_id(target_id))
    }

    pub(super) fn remove_files_by_ids(&mut self, target_ids: &HashSet<String>) -> bool {
        let before = self.files.len();
        self.files.retain(|file| !target_ids.contains(&file.id));
        let mut changed = self.files.len() != before;
        for child in &mut self.children {
            changed |= child.remove_files_by_ids(target_ids);
        }
        changed
    }

    pub(super) fn set_file_rating(
        &mut self,
        file_id: &str,
        rating: wavecrate::sample_sources::Rating,
        locked: bool,
    ) -> bool {
        for file in &mut self.files {
            if file.id == file_id {
                file.rating = rating;
                file.rating_locked = locked;
                return true;
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.set_file_rating(file_id, rating, locked))
    }

    pub(super) fn set_file_last_played_at(&mut self, file_id: &str, last_played_at: i64) -> bool {
        if self.set_direct_file_last_played_at(file_id, last_played_at) {
            return true;
        }
        self.children
            .iter_mut()
            .any(|child| child.set_file_last_played_at(file_id, last_played_at))
    }

    pub(super) fn set_direct_file_last_played_at(
        &mut self,
        file_id: &str,
        last_played_at: i64,
    ) -> bool {
        for file in &mut self.files {
            if file.id == file_id {
                file.set_last_played_at(Some(last_played_at));
                return true;
            }
        }
        false
    }

    pub(super) fn set_file_last_curated_at(&mut self, file_id: &str, last_curated_at: i64) -> bool {
        for file in &mut self.files {
            if file.id == file_id {
                file.set_last_curated_at(Some(last_curated_at));
                return true;
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.set_file_last_curated_at(file_id, last_curated_at))
    }

    pub(super) fn set_files_last_curated_at(
        &mut self,
        target_ids: &HashSet<String>,
        last_curated_at: i64,
    ) -> bool {
        let mut changed = false;
        for file in &mut self.files {
            if target_ids.contains(&file.id) {
                file.set_last_curated_at(Some(last_curated_at));
                changed = true;
            }
        }
        for child in &mut self.children {
            changed |= child.set_files_last_curated_at(target_ids, last_curated_at);
        }
        changed
    }

    pub(super) fn set_file_collection(
        &mut self,
        file_id: &str,
        collection: wavecrate::sample_sources::SampleCollection,
    ) -> bool {
        for file in &mut self.files {
            if file.id == file_id {
                return file.add_collection(collection);
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.set_file_collection(file_id, collection))
    }

    pub(super) fn remove_file_collection(
        &mut self,
        file_id: &str,
        collection: wavecrate::sample_sources::SampleCollection,
    ) -> bool {
        for file in &mut self.files {
            if file.id == file_id {
                return file.remove_collection(collection);
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.remove_file_collection(file_id, collection))
    }
}
