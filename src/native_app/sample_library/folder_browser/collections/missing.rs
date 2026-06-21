use std::{collections::BTreeMap, path::Path};

use wavecrate::sample_sources::{SampleCollection, SourceDatabase, WavEntry};

use super::super::{FileEntry, FolderBrowserState, SourceEntry};
use super::model::MissingCollectionFile;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MissingCollectionSnapshot {
    files: Vec<FileEntry>,
    counts: BTreeMap<u8, usize>,
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
        let path_id = path.to_string_lossy();
        self.selected_audio_files()
            .into_iter()
            .any(|file| file.id == path_id && file.is_missing())
    }

    pub(in crate::native_app) fn active_collection(&self) -> Option<SampleCollection> {
        self.selection.selected_collection
    }

    pub(in crate::native_app) fn missing_collection_file_for_path(
        &self,
        path: &Path,
        collection: SampleCollection,
    ) -> Option<MissingCollectionFile> {
        let path_id = path.to_string_lossy();
        self.sample_list
            .missing_collection_files
            .iter()
            .find(|file| file.id == path_id && file.belongs_to_collection(collection))
            .and_then(|file| self.missing_collection_file_from_entry(file, collection))
    }

    pub(in crate::native_app) fn missing_collection_files_for_collection(
        &self,
        collection: SampleCollection,
    ) -> Vec<MissingCollectionFile> {
        self.sample_list
            .missing_collection_files
            .iter()
            .filter(|file| file.belongs_to_collection(collection))
            .filter_map(|file| self.missing_collection_file_from_entry(file, collection))
            .collect()
    }

    fn missing_collection_file_from_entry(
        &self,
        file: &FileEntry,
        collection: SampleCollection,
    ) -> Option<MissingCollectionFile> {
        let absolute_path = Path::new(&file.id);
        let (root, relative_path) = self.source_relative_file_path(absolute_path)?;
        Some(MissingCollectionFile {
            root,
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
        let Ok(db) = SourceDatabase::open_read_only(&source.root) else {
            return;
        };
        let Ok(entries) = db.list_files() else {
            return;
        };
        for entry in entries {
            let collections = db
                .collections_for_path(&entry.relative_path)
                .unwrap_or_default();
            if collections.is_empty() || source_entry_is_present(source, &entry) {
                continue;
            }
            for collection in &collections {
                *self.counts.entry(collection.index()).or_insert(0) += 1;
            }
            if active_collection.is_some_and(|collection| collections.contains(&collection)) {
                self.files.push(FileEntry::missing_collection_member(
                    &source.root.join(&entry.relative_path),
                    entry.tag,
                    entry.locked,
                    collections,
                    entry.last_played_at,
                ));
            }
        }
    }
}

fn source_entry_is_present(source: &SourceEntry, entry: &WavEntry) -> bool {
    source.root.join(&entry.relative_path).is_file()
}
