use super::{FolderBrowserUiState, Path, PathBuf};
#[cfg(test)]
use crate::app_core::state::FolderRowView;

fn normalize_folder_name_input(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(String::from("Folder name cannot be empty"));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(String::from("Folder name is invalid"));
    }
    if trimmed.contains(['/', '\\']) {
        return Err(String::from("Folder name cannot contain path separators"));
    }
    Ok(trimmed.to_string())
}

fn display_relative_folder_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn folder_exists_in_rows(folder_ui: &FolderBrowserUiState, relative_path: &Path) -> bool {
    folder_ui.rows.iter().any(|row| row.path == relative_path)
}

pub(super) fn folder_create_validation_error(
    folder_ui: &FolderBrowserUiState,
    parent: &Path,
    name: &str,
) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let relative = if parent.as_os_str().is_empty() {
        PathBuf::from(&normalized)
    } else {
        parent.join(&normalized)
    };
    folder_exists_in_rows(folder_ui, &relative).then_some(format!(
        "Folder already exists: {}",
        display_relative_folder_path(&relative)
    ))
}

pub(super) fn folder_rename_validation_error(
    folder_ui: &FolderBrowserUiState,
    target: &Path,
    name: &str,
) -> Option<String> {
    let normalized = match normalize_folder_name_input(name) {
        Ok(normalized) => normalized,
        Err(err) => return Some(err),
    };
    let renamed = folder_with_name(target, &normalized);
    if renamed == target {
        return None;
    }
    folder_exists_in_rows(folder_ui, &renamed).then_some(format!(
        "Folder already exists: {}",
        display_relative_folder_path(&renamed)
    ))
}

fn folder_with_name(target: &Path, name: &str) -> PathBuf {
    target.parent().map_or_else(
        || PathBuf::from(name),
        |parent| {
            if parent.as_os_str().is_empty() {
                PathBuf::from(name)
            } else {
                parent.join(name)
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder_ui_with_paths(paths: &[&str]) -> FolderBrowserUiState {
        let mut ui = FolderBrowserUiState::default();
        for path in paths {
            ui.rows.push(FolderRowView {
                path: PathBuf::from(path),
                name: Path::new(path)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(path)
                    .to_string(),
                depth: path.matches('/').count() + 1,
                has_children: false,
                expanded: false,
                selected: false,
                negated: false,
                hotkey: None,
                is_root: path.is_empty(),
                file_scope_mode: None,
            });
        }
        ui
    }

    #[test]
    fn folder_name_normalization_rejects_empty_reserved_and_path_values() {
        assert_eq!(
            normalize_folder_name_input("  "),
            Err(String::from("Folder name cannot be empty"))
        );
        assert_eq!(
            normalize_folder_name_input(".."),
            Err(String::from("Folder name is invalid"))
        );
        assert_eq!(
            normalize_folder_name_input("bad/name"),
            Err(String::from("Folder name cannot contain path separators"))
        );
        assert_eq!(
            normalize_folder_name_input("  Fresh Folder  "),
            Ok(String::from("Fresh Folder"))
        );
    }

    #[test]
    fn folder_validation_reports_create_and_rename_collisions() {
        let ui = folder_ui_with_paths(&["", "drums", "drums/existing", "kicks"]);

        assert_eq!(
            folder_create_validation_error(&ui, Path::new("drums"), "existing"),
            Some(String::from("Folder already exists: drums/existing"))
        );
        assert_eq!(
            folder_rename_validation_error(&ui, Path::new("drums"), "kicks"),
            Some(String::from("Folder already exists: kicks"))
        );
        assert_eq!(
            folder_rename_validation_error(&ui, Path::new("drums"), "drums"),
            None
        );
    }
}
