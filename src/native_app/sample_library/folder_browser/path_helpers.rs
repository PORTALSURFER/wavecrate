use std::path::Path;

use radiant::prelude as ui;

const FOLDER_RENAME_INPUT_SCOPE: u64 = 0x5743_0000_0000_4601;
const FILE_RENAME_INPUT_SCOPE: u64 = 0x5743_0000_0000_4602;

pub(super) fn path_id(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(super) fn path_id_matches(id: &str, path: &Path) -> bool {
    id == path_id(path) || Path::new(id) == path
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
    ui::stable_widget_id(FOLDER_RENAME_INPUT_SCOPE, folder_id)
}

pub(super) fn file_rename_input_id(file_id: &str) -> u64 {
    ui::stable_widget_id(FILE_RENAME_INPUT_SCOPE, file_id)
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
