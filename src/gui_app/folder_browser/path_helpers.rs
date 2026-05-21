use std::path::Path;

const FOLDER_RENAME_INPUT_BASE_ID: u64 = 70_000_000;
const FILE_RENAME_INPUT_BASE_ID: u64 = 80_000_000;

pub(super) fn path_id(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(super) fn rewrite_path_id(id: &str, old_path: &Path, new_path: &Path) -> String {
    let path = Path::new(id);
    if path == old_path {
        return path_id(new_path);
    }
    path.strip_prefix(old_path)
        .map(|relative| path_id(&new_path.join(relative)))
        .unwrap_or_else(|_| id.to_string())
}

pub(super) fn valid_folder_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
}

pub(super) fn valid_file_name(name: &str) -> bool {
    valid_folder_name(name)
}

pub(super) fn next_available_folder_name(parent: &Path) -> String {
    const BASE_NAME: &str = "New folder";
    if !parent.join(BASE_NAME).exists() {
        return String::from(BASE_NAME);
    }
    (2..)
        .map(|index| format!("{BASE_NAME} {index}"))
        .find(|name| !parent.join(name).exists())
        .unwrap_or_else(|| String::from(BASE_NAME))
}

pub(super) fn resolved_file_rename(old_path: &Path, submitted: &str) -> Option<String> {
    if submitted.is_empty() {
        return None;
    }
    let submitted_path = Path::new(submitted);
    if submitted_path.components().count() != 1 {
        return None;
    }
    let extension = old_path.extension()?.to_string_lossy();
    Some(format!("{submitted}.{extension}"))
}

pub(super) fn file_rename_draft(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string())
}

pub(super) fn rename_input_id(folder_id: &str) -> u64 {
    folder_id
        .bytes()
        .fold(FOLDER_RENAME_INPUT_BASE_ID, |hash, byte| {
            hash.wrapping_mul(16_777_619) ^ u64::from(byte)
        })
}

pub(super) fn file_rename_input_id(file_id: &str) -> u64 {
    file_id
        .bytes()
        .fold(FILE_RENAME_INPUT_BASE_ID, |hash, byte| {
            hash.wrapping_mul(16_777_619) ^ u64::from(byte)
        })
}

pub(super) fn folder_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn file_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

pub(super) fn file_stem_label(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| file_label(path))
}

pub(super) fn file_extension_label(path: &Path) -> String {
    path.extension()
        .map(|extension| extension.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(super) fn offset_index(current: usize, delta: i32, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        current
            .saturating_add(delta as usize)
            .min(len.saturating_sub(1))
    }
}
