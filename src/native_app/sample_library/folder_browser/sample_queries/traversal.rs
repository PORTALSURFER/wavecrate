use std::collections::HashMap;
use wavecrate::sample_sources::SampleCollection;

use super::filters::{audio_file_matches_name_query, audio_file_matches_parsed_tags};
use super::{FileEntry, FolderEntry};

pub(super) fn collect_audio_files<'a>(folder: &'a FolderEntry, files: &mut Vec<&'a FileEntry>) {
    files.extend(folder.files.iter().filter(|file| file.is_audio()));
    for child in &folder.children {
        collect_audio_files(child, files);
    }
}

pub(super) fn collect_collection_audio_files<'a>(
    folder: &'a FolderEntry,
    collection: SampleCollection,
    files: &mut Vec<&'a FileEntry>,
) {
    files.extend(
        folder
            .files
            .iter()
            .filter(|file| file.is_audio() && file.belongs_to_collection(collection)),
    );
    for child in &folder.children {
        collect_collection_audio_files(child, collection, files);
    }
}

pub(super) fn count_matching_audio_files_in_folder(
    folder: &FolderEntry,
    name_query: &str,
    required_tags: &[String],
    tags_by_file: &HashMap<String, Vec<String>>,
    collection: Option<SampleCollection>,
) -> usize {
    let local_count = folder
        .files
        .iter()
        .filter(|file| {
            file.is_audio()
                && collection.is_none_or(|collection| file.belongs_to_collection(collection))
                && audio_file_matches_name_query(file, name_query)
                && audio_file_matches_parsed_tags(file, tags_by_file, required_tags)
        })
        .count();
    local_count
        + folder
            .children
            .iter()
            .map(|child| {
                count_matching_audio_files_in_folder(
                    child,
                    name_query,
                    required_tags,
                    tags_by_file,
                    collection,
                )
            })
            .sum::<usize>()
}
