use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use wavecrate::sample_sources::SampleCollection;

use crate::native_app::sample_library::folder_browser::scanning::SourceMetadataMap;

use super::super::{FileEntry, FolderBrowserState, FolderEntry, SourceEntry, path_id_matches};
use super::model::MissingCollectionFile;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::native_app::sample_library::folder_browser) struct MissingCollectionSnapshot {
    pub(super) files: Vec<FileEntry>,
    pub(super) counts: BTreeMap<u8, usize>,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn refresh_missing_collection_state(&mut self) {
        let snapshot = MissingCollectionSnapshot::for_sources(
            &self.source.sources,
            self.selection.selected_collection,
        );
        let changed = self.sample_list.missing_collection_files != snapshot.files
            || self.sample_list.missing_collection_counts != snapshot.counts;
        if !changed {
            return;
        }
        self.sample_list.missing_collection_files = snapshot.files;
        self.sample_list.missing_collection_counts = snapshot.counts;
        self.bump_file_content_revision();
    }

    pub(in crate::native_app) fn context_file_is_missing(&self, path: &Path) -> bool {
        self.selected_audio_files()
            .into_iter()
            .any(|file| path_id_matches(&file.id, path) && file.is_missing())
    }

    pub(in crate::native_app) fn active_collection(&self) -> Option<SampleCollection> {
        self.selection.selected_collection
    }

    pub(in crate::native_app) fn missing_collection_file_for_path(
        &self,
        path: &Path,
        collection: SampleCollection,
    ) -> Option<MissingCollectionFile> {
        self.source
            .sources
            .iter()
            .flat_map(|source| source.missing_collection_snapshot.files.iter())
            .find(|file| path_id_matches(&file.id, path) && file.belongs_to_collection(collection))
            .and_then(|file| self.missing_collection_file_from_entry(file, collection))
    }

    pub(in crate::native_app) fn missing_collection_files_for_collection(
        &self,
        collection: SampleCollection,
    ) -> Vec<MissingCollectionFile> {
        self.source
            .sources
            .iter()
            .flat_map(|source| source.missing_collection_snapshot.files.iter())
            .filter(|file| file.belongs_to_collection(collection))
            .filter_map(|file| self.missing_collection_file_from_entry(file, collection))
            .collect()
    }

    pub(in crate::native_app) fn remove_missing_collection_files(
        &mut self,
        files: &[MissingCollectionFile],
    ) -> bool {
        let mut changed = false;
        for file in files {
            if let Some(source) = self
                .source
                .sources
                .iter_mut()
                .find(|source| source.root == file.root)
            {
                changed |= source
                    .missing_collection_snapshot
                    .remove_path(&file.absolute_path);
            }
        }
        if changed {
            self.refresh_missing_collection_state();
        }
        changed
    }

    fn missing_collection_file_from_entry(
        &self,
        file: &FileEntry,
        collection: SampleCollection,
    ) -> Option<MissingCollectionFile> {
        let absolute_path = Path::new(&file.id);
        let (root, database_root, relative_path) =
            self.source_database_relative_file_path(absolute_path)?;
        Some(MissingCollectionFile {
            root,
            database_root,
            relative_path,
            absolute_path: absolute_path.to_path_buf(),
            collection,
        })
    }
}

impl MissingCollectionSnapshot {
    fn for_sources(sources: &[SourceEntry], active_collection: Option<SampleCollection>) -> Self {
        let mut snapshot = Self::default();
        for source in sources {
            snapshot.collect_source(source, active_collection);
        }
        snapshot.files.sort_by(|left, right| {
            left.name_sort_key()
                .cmp(&right.name_sort_key())
                .then_with(|| left.id.cmp(&right.id))
        });
        snapshot
    }

    fn collect_source(
        &mut self,
        source: &SourceEntry,
        active_collection: Option<SampleCollection>,
    ) {
        for (collection, count) in &source.missing_collection_snapshot.counts {
            *self.counts.entry(*collection).or_insert(0) += count;
        }
        let Some(collection) = active_collection else {
            return;
        };
        self.files.extend(
            source
                .missing_collection_snapshot
                .files
                .iter()
                .filter(|file| file.belongs_to_collection(collection))
                .cloned(),
        );
    }

    pub(in crate::native_app::sample_library::folder_browser) fn from_source_metadata(
        root: &Path,
        root_folder: &FolderEntry,
        metadata: &SourceMetadataMap,
    ) -> Self {
        let mut present_relative_paths = HashSet::new();
        collect_present_relative_paths(root, root_folder, &mut present_relative_paths);

        let mut snapshot = Self::default();
        for (relative_path, (rating, locked, collections, last_played_at, last_curated_at)) in
            metadata
        {
            if collections.is_empty() || present_relative_paths.contains(relative_path) {
                continue;
            }
            snapshot.push_missing_file(FileEntry::missing_collection_member(
                &root.join(relative_path),
                *rating,
                *locked,
                collections.clone(),
                *last_played_at,
                *last_curated_at,
            ));
        }
        snapshot.sort_files();
        snapshot
    }

    pub(in crate::native_app::sample_library::folder_browser) fn add_missing_file(
        &mut self,
        file: FileEntry,
    ) -> bool {
        if file.collection_memberships().is_empty() {
            return false;
        }
        let missing = FileEntry::missing_collection_member_from_file(Path::new(&file.id), &file);
        self.remove_path(Path::new(&missing.id));
        self.push_missing_file(missing);
        self.sort_files();
        true
    }

    pub(in crate::native_app::sample_library::folder_browser) fn add_missing_files_from_folder(
        &mut self,
        folder: &FolderEntry,
    ) -> bool {
        let before = self.files.len();
        for file in folder.all_files() {
            self.add_missing_file(file.clone());
        }
        self.files.len() != before
    }

    pub(in crate::native_app::sample_library::folder_browser) fn remove_path(
        &mut self,
        path: &Path,
    ) -> bool {
        let before = self.files.len();
        self.files.retain(|file| !path_id_matches(&file.id, path));
        if self.files.len() == before {
            return false;
        }
        self.rebuild_counts();
        true
    }

    pub(in crate::native_app::sample_library::folder_browser) fn remove_prefix(
        &mut self,
        path: &Path,
    ) -> bool {
        let before = self.files.len();
        self.files
            .retain(|file| !Path::new(&file.id).starts_with(path));
        if self.files.len() == before {
            return false;
        }
        self.rebuild_counts();
        true
    }

    fn push_missing_file(&mut self, file: FileEntry) {
        for collection in file.collection_memberships() {
            *self.counts.entry(collection.index()).or_insert(0) += 1;
        }
        self.files.push(file);
    }

    fn rebuild_counts(&mut self) {
        self.counts.clear();
        for file in &self.files {
            for collection in file.collection_memberships() {
                *self.counts.entry(collection.index()).or_insert(0) += 1;
            }
        }
    }

    fn sort_files(&mut self) {
        self.files.sort_by(|left, right| {
            left.name_sort_key()
                .cmp(&right.name_sort_key())
                .then_with(|| left.id.cmp(&right.id))
        });
    }
}

fn collect_present_relative_paths(root: &Path, folder: &FolderEntry, paths: &mut HashSet<PathBuf>) {
    for file in &folder.files {
        if let Ok(relative) = Path::new(&file.id).strip_prefix(root) {
            paths.insert(relative.to_path_buf());
        }
    }
    for child in &folder.children {
        collect_present_relative_paths(root, child, paths);
    }
}
