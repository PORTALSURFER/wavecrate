use std::{
    fs,
    path::{Path, PathBuf},
};

use super::super::{
    FileEntry,
    file_model::{last_played_label, last_played_rank},
    path_helpers::{file_extension_label, file_label, file_stem_label, path_id},
};
use wavecrate::sample_sources::{Rating, SampleCollection};

pub(in crate::native_app::sample_library::folder_browser) fn file_entry(
    path: &PathBuf,
) -> FileEntry {
    file_entry_with_metadata(path, Rating::NEUTRAL, false, Vec::new(), None, None)
}

pub(in crate::native_app::sample_library::folder_browser) fn file_entry_with_metadata(
    path: &PathBuf,
    rating: Rating,
    rating_locked: bool,
    collections: Vec<SampleCollection>,
    last_played_at: Option<i64>,
    last_curated_at: Option<i64>,
) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len).unwrap_or_default();
    FileEntry {
        id: path_id(path),
        name: file_label(path),
        stem: file_stem_label(path),
        extension: file_extension_label(path),
        kind: file_kind(path),
        size: format_size(size_bytes),
        size_bytes,
        modified: last_played_label(last_played_at),
        modified_rank: last_played_rank(last_played_at),
        rating,
        rating_locked,
        last_curated_at,
        collection: collections.first().copied(),
        collections,
    }
}

fn file_kind(path: &Path) -> String {
    if wavecrate_library::sample_sources::is_supported_audio(path) {
        return String::from("Audio");
    }
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") if wavecrate_library::sample_sources::is_apple_double_sidecar(path) => {
            String::from("File")
        }
        Some("aif" | "aiff" | "flac" | "mp3") => String::from("Unsupported audio"),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp") => String::from("Image"),
        Some("json" | "txt" | "md" | "toml" | "rs") => String::from("Text"),
        _ => String::from("File"),
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} B")
    }
}
