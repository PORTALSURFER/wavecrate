use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use super::super::{
    FileEntry,
    path_helpers::{file_extension_label, file_label, file_stem_label, path_id},
};
use wavecrate::sample_sources::{Rating, SampleCollection};

pub(in crate::native_app::library_browser::folder_browser) fn file_entry(
    path: &PathBuf,
) -> FileEntry {
    file_entry_with_metadata(path, Rating::NEUTRAL, false, Vec::new())
}

pub(in crate::native_app::library_browser::folder_browser) fn file_entry_with_metadata(
    path: &PathBuf,
    rating: Rating,
    rating_locked: bool,
    collections: Vec<SampleCollection>,
) -> FileEntry {
    let metadata = fs::metadata(path).ok();
    let size_bytes = metadata.as_ref().map(fs::Metadata::len).unwrap_or_default();
    let modified = metadata.and_then(|metadata| metadata.modified().ok());
    FileEntry {
        id: path_id(path),
        name: file_label(path),
        stem: file_stem_label(path),
        extension: file_extension_label(path),
        kind: file_kind(path),
        size: format_size(size_bytes),
        size_bytes,
        modified: modified_label(modified),
        modified_rank: modified_rank(modified),
        rating,
        rating_locked,
        collection: collections.first().copied(),
        collections,
    }
}

fn file_kind(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("wav") => String::from("Audio"),
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

fn modified_label(modified: Option<SystemTime>) -> String {
    let Some(modified) = modified else {
        return String::from("-");
    };
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    let days = age.as_secs() / 86_400;
    if days == 0 {
        String::from("Today")
    } else if days == 1 {
        String::from("1 day")
    } else {
        format!("{days} days")
    }
}

fn modified_rank(modified: Option<SystemTime>) -> u64 {
    modified
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map(|age| age.as_secs())
        .unwrap_or(u64::MAX)
}
