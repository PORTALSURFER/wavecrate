use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::native_app::app::{NativeAppState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target as context_menu;
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};
use crate::native_app::sample_library::file_actions::sample_path_label;

impl NativeAppState {
    pub(in crate::native_app) fn duplicate_context_sample_same(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.ui.browser_interaction.context_menu.take() else {
            return;
        };
        match self.duplicate_context_sample_same_path(&menu) {
            Ok(destination) => {
                let duplicated_id = destination.display().to_string();
                self.library
                    .folder_browser
                    .refresh_file_path_across_sources(&destination);
                self.library.folder_browser.select_file(duplicated_id);
                self.library
                    .folder_browser
                    .follow_selected_file_view_matching_tags(12, 6, 2, &self.metadata.tags_by_file);
                self.ui.status.sample = format!("Duplicated {}", sample_path_label(&destination));
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_same",
                    Some("browser"),
                    Some(context_menu::target_label(&destination).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = error.clone();
                emit_gui_action(
                    "browser.context_menu.sample.duplicate_same",
                    Some("browser"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn duplicate_context_sample_same_path(
        &self,
        menu: &BrowserContextMenu,
    ) -> Result<PathBuf, String> {
        if menu.kind != BrowserContextTargetKind::Sample {
            return Err(String::from("Choose a sample to duplicate"));
        }
        if menu.sample_missing || !menu.path.is_file() {
            return Err(String::from("Sample file is missing"));
        }
        let Some((source_root, source_database_root)) =
            self.context_sample_source_roots(menu.source_id.as_deref(), &menu.path)
        else {
            return Err(String::from("Sample source is unavailable"));
        };
        let destination = duplicate_same_destination(&menu.path);
        duplicate_sample_file_with_metadata(
            &menu.path,
            &destination,
            &source_root,
            &source_database_root,
        )?;
        Ok(destination)
    }

    fn context_sample_source_roots(
        &self,
        source_id: Option<&str>,
        path: &Path,
    ) -> Option<(PathBuf, PathBuf)> {
        source_id
            .and_then(|source_id| self.library.folder_browser.source_roots(source_id))
            .or_else(|| self.library.folder_browser.source_roots_for_path(path))
    }
}

fn duplicate_same_destination(source: &Path) -> PathBuf {
    let parent = source.parent().unwrap_or_else(|| Path::new(""));
    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = source
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    for count in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_copy{count:03}.{extension}"),
            None => format!("{stem}_copy{count:03}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded copy suffix search should find a destination")
}

fn duplicate_sample_file_with_metadata(
    source: &Path,
    destination: &Path,
    source_root: &Path,
    source_database_root: &Path,
) -> Result<(), String> {
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|err| format!("Duplicate failed: {err}"))?;
    if let Err(error) = wavecrate::sample_sources::persist_copied_file_metadata(
        source_root,
        source_database_root,
        source,
        destination,
    ) {
        let _ = std::fs::remove_file(destination);
        return Err(error);
    }
    Ok(())
}
