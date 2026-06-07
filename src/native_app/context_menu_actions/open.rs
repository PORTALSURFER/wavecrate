use radiant::gui::types::Point;
use std::path::Path;
use std::time::Instant;

use super::super::context_menu::{self, BrowserContextMenu, BrowserContextTargetKind};
use super::super::file_actions::sample_path_label;
use crate::native_app::app_scope::{NativeAppState, emit_gui_action};

impl NativeAppState {
    pub(in crate::native_app) fn open_source_context_menu(
        &mut self,
        source_id: String,
        position: Point,
    ) {
        let started_at = Instant::now();
        let Some(path) = self.folder_browser.source_root_path(&source_id) else {
            self.sample_status = String::from("Source is unavailable");
            emit_gui_action(
                "browser.context_menu.source.open",
                Some("sources"),
                None,
                "error",
                started_at,
                Some("source unavailable"),
            );
            return;
        };
        let title = context_menu_title(&path);
        let source_removable = self.folder_browser.source_is_removable(&source_id);
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Source,
            path,
            source_id: Some(source_id),
            source_removable,
            metadata_tag: None,
            collection: None,
            anchor: position,
            title,
        });
    }

    pub(in crate::native_app) fn open_folder_context_menu(
        &mut self,
        folder_id: String,
        position: Point,
    ) {
        let started_at = Instant::now();
        let Some(path) = self.folder_browser.folder_path(&folder_id) else {
            self.sample_status = String::from("Folder is unavailable");
            emit_gui_action(
                "browser.context_menu.folder.open",
                Some("folder_browser"),
                None,
                "error",
                started_at,
                Some("folder unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Folder, &path) {
            self.sample_status = String::from("Folder is missing");
            emit_gui_action(
                "browser.context_menu.folder.open",
                Some("folder_browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("folder missing"),
            );
            return;
        }
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Folder,
            title: context_menu_title(&path),
            path,
            source_id: None,
            source_removable: false,
            metadata_tag: None,
            collection: None,
            anchor: position,
        });
    }

    pub(in crate::native_app) fn open_sample_context_menu(
        &mut self,
        path: String,
        position: Point,
    ) {
        let started_at = Instant::now();
        self.folder_browser
            .focus_file_preserving_selection(path.clone());
        let Some(path) = self.folder_browser.context_sample_path(&path) else {
            self.sample_status = String::from("Sample is unavailable");
            emit_gui_action(
                "browser.context_menu.sample.open",
                Some("browser"),
                None,
                "error",
                started_at,
                Some("sample unavailable"),
            );
            return;
        };
        if !context_menu::target_available(&BrowserContextTargetKind::Sample, &path) {
            self.sample_status = String::from("Sample file is missing");
            emit_gui_action(
                "browser.context_menu.sample.open",
                Some("browser"),
                Some(context_menu::target_label(&path).as_str()),
                "error",
                started_at,
                Some("sample missing"),
            );
            return;
        }
        let collection = self
            .folder_browser
            .active_collection_for_context_file(&path);
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Sample,
            title: sample_path_label(&path),
            path,
            source_id: None,
            source_removable: false,
            metadata_tag: None,
            collection,
            anchor: position,
        });
    }

    pub(in crate::native_app) fn open_metadata_tag_context_menu(
        &mut self,
        tag: String,
        position: Point,
    ) {
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::MetadataTag,
            path: Path::new("").to_path_buf(),
            source_id: None,
            source_removable: false,
            metadata_tag: Some(tag.clone()),
            collection: None,
            anchor: position,
            title: tag,
        });
    }
}

fn context_menu_title(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
