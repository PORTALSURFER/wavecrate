use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use super::{FileEntry, file_entry, folder_label, path_id, rewrite_path_id};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::gui_app) struct FolderEntry {
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

    pub(super) fn has_children(&self) -> bool {
        !self.children.is_empty()
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
                *file = file_entry(&PathBuf::from(new_path));
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

    pub(super) fn set_file_collection(
        &mut self,
        file_id: &str,
        collection: wavecrate::sample_sources::SampleCollection,
    ) -> bool {
        for file in &mut self.files {
            if file.id == file_id {
                file.collection = Some(collection);
                return true;
            }
        }
        self.children
            .iter_mut()
            .any(|child| child.set_file_collection(file_id, collection))
    }
}
