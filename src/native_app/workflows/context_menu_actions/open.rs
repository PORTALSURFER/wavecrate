use radiant::gui::types::Point;
use std::path::Path;
use std::time::Instant;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action};
use crate::native_app::sample_library::context_menu_target as context_menu;
use crate::native_app::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextPointerAnchor, BrowserContextPointerTarget,
    BrowserContextTargetKind,
};
use crate::native_app::sample_library::file_actions::sample_path_label;
use crate::native_app::sample_library::folder_browser::view_contract::collection_hotkey;
use wavecrate::sample_sources::SampleCollection;
use wavecrate::sample_sources::SourceRole;

const SAMPLE_CONTEXT_SHORTCUT_ANCHOR: Point = Point { x: 720.0, y: 520.0 };
const COLLECTION_CONTEXT_SHORTCUT_ANCHOR: Point = Point { x: 72.0, y: 720.0 };
const FOLDER_CONTEXT_SHORTCUT_ANCHOR: Point = Point { x: 96.0, y: 240.0 };
const SOURCE_CONTEXT_SHORTCUT_ANCHOR: Point = Point { x: 96.0, y: 120.0 };

impl NativeAppState {
    pub(in crate::native_app) fn open_context_menu_from_shortcut(
        &mut self,
        context: &radiant::prelude::UiUpdateContext<GuiMessage>,
    ) {
        if self
            .ui
            .browser_interaction
            .waveform_context_menu
            .take()
            .is_some()
        {
            return;
        }
        if self
            .waveform
            .current
            .play_selection_context_menu_anchor()
            .is_some()
        {
            self.open_play_selection_context_menu_from_shortcut(context.current_pointer_position());
            return;
        }
        if self.close_open_context_menu() {
            return;
        }
        if let Some(file_id) = self
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned)
        {
            let target = BrowserContextPointerTarget::Sample(file_id.clone());
            let anchor = self.context_menu_pointer_position_for(
                &target,
                SAMPLE_CONTEXT_SHORTCUT_ANCHOR,
                context.current_pointer_position(),
            );
            self.open_sample_context_menu(file_id, anchor);
            return;
        }
        if let Some(collection) = self.library.folder_browser.selected_collection() {
            let target = BrowserContextPointerTarget::Collection(collection);
            let anchor = self.context_menu_pointer_position_for(
                &target,
                COLLECTION_CONTEXT_SHORTCUT_ANCHOR,
                context.current_pointer_position(),
            );
            self.open_collection_context_menu(collection, anchor);
            return;
        }
        if let Some(folder_id) = self
            .library
            .folder_browser
            .selected_folder_id()
            .map(str::to_owned)
        {
            let target = BrowserContextPointerTarget::Folder(folder_id.clone());
            let anchor = self.context_menu_pointer_position_for(
                &target,
                FOLDER_CONTEXT_SHORTCUT_ANCHOR,
                context.current_pointer_position(),
            );
            self.open_folder_context_menu(folder_id, anchor);
            return;
        }
        let source_id = self.library.folder_browser.selected_source_id().to_owned();
        let target = BrowserContextPointerTarget::Source(source_id.clone());
        let anchor = self.context_menu_pointer_position_for(
            &target,
            SOURCE_CONTEXT_SHORTCUT_ANCHOR,
            context.current_pointer_position(),
        );
        self.open_source_context_menu(source_id, anchor);
    }

    fn close_open_context_menu(&mut self) -> bool {
        let open = self.ui.browser_interaction.context_menu.is_some()
            || self.ui.browser_interaction.waveform_context_menu.is_some();
        if open {
            self.ui.browser_interaction.context_menu = None;
            self.ui.browser_interaction.waveform_context_menu = None;
        }
        open
    }

    pub(in crate::native_app) fn open_source_context_menu(
        &mut self,
        source_id: String,
        position: Point,
    ) {
        self.remember_context_menu_pointer_anchor(
            BrowserContextPointerTarget::Source(source_id.clone()),
            position,
        );
        let started_at = Instant::now();
        let Some(path) = self.library.folder_browser.source_root_path(&source_id) else {
            self.ui.status.sample = String::from("Source is unavailable");
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
        let source_removable = self.library.folder_browser.source_is_removable(&source_id);
        let source_role = self
            .library
            .folder_browser
            .source_role(&source_id)
            .unwrap_or(SourceRole::Normal);
        self.ui.browser_interaction.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Source,
            path,
            source_id: Some(source_id),
            source_role,
            source_removable,
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: None,
            collection: None,
            sample_missing: false,
            sample_keep_locked: false,
            anchor: position,
            title,
        });
    }

    pub(in crate::native_app) fn open_folder_context_menu(
        &mut self,
        folder_id: String,
        position: Point,
    ) {
        self.remember_context_menu_pointer_anchor(
            BrowserContextPointerTarget::Folder(folder_id.clone()),
            position,
        );
        let started_at = Instant::now();
        let Some(path) = self.library.folder_browser.folder_path(&folder_id) else {
            self.ui.status.sample = String::from("Folder is unavailable");
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
            self.ui.status.sample = String::from("Folder is missing");
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
        self.ui.browser_interaction.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Folder,
            title: context_menu_title(&path),
            folder_locked: self
                .library
                .folder_browser
                .folder_exactly_locked(&folder_id),
            folder_lock_inherited: self
                .library
                .folder_browser
                .folder_lock_inherited(&folder_id),
            path,
            source_id: None,
            source_role: SourceRole::Normal,
            source_removable: false,
            metadata_tag: None,
            collection: None,
            sample_missing: false,
            sample_keep_locked: false,
            anchor: position,
        });
    }

    pub(in crate::native_app) fn open_collection_context_menu(
        &mut self,
        collection: SampleCollection,
        position: Point,
    ) {
        self.remember_context_menu_pointer_anchor(
            BrowserContextPointerTarget::Collection(collection),
            position,
        );
        let title = self
            .library
            .folder_browser
            .visible_collections()
            .into_iter()
            .find(|entry| entry.collection == collection)
            .map(|entry| entry.name)
            .unwrap_or_else(|| format!("Collection {}", collection_hotkey(collection)));
        self.ui.browser_interaction.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Collection,
            path: Path::new("").to_path_buf(),
            source_id: None,
            source_role: SourceRole::Normal,
            source_removable: false,
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: None,
            collection: Some(collection),
            sample_missing: false,
            sample_keep_locked: false,
            anchor: position,
            title,
        });
    }

    pub(in crate::native_app) fn open_sample_context_menu(
        &mut self,
        path: String,
        position: Point,
    ) {
        self.remember_context_menu_pointer_anchor(
            BrowserContextPointerTarget::Sample(path.clone()),
            position,
        );
        let started_at = Instant::now();
        if !self
            .library
            .folder_browser
            .explicit_multi_file_selection_active()
            || self.library.folder_browser.is_file_selected(&path)
        {
            self.library
                .folder_browser
                .focus_file_preserving_selection(path.clone());
        }
        let Some(path) = self.library.folder_browser.context_sample_path(&path) else {
            self.ui.status.sample = String::from("Sample is unavailable");
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
            self.ui.status.sample = String::from("Sample file is missing");
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
            .library
            .folder_browser
            .active_collection_for_context_file(&path);
        let sample_missing = self.library.folder_browser.context_file_is_missing(&path);
        let sample_keep_locked = self
            .library
            .folder_browser
            .context_file_is_keep_locked(&path);
        self.ui.browser_interaction.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Sample,
            title: sample_path_label(&path),
            path,
            source_id: None,
            source_role: SourceRole::Normal,
            source_removable: false,
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: None,
            collection,
            sample_missing,
            sample_keep_locked,
            anchor: position,
        });
    }

    pub(in crate::native_app) fn open_metadata_tag_context_menu(
        &mut self,
        tag: String,
        position: Point,
    ) {
        self.ui.browser_interaction.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::MetadataTag,
            path: Path::new("").to_path_buf(),
            source_id: None,
            source_role: SourceRole::Normal,
            source_removable: false,
            folder_locked: false,
            folder_lock_inherited: false,
            metadata_tag: Some(tag.clone()),
            collection: None,
            sample_missing: false,
            sample_keep_locked: false,
            anchor: position,
            title: tag,
        });
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn remember_context_menu_pointer_anchor(
        &mut self,
        target: BrowserContextPointerTarget,
        position: Point,
    ) {
        if let Some(anchor) = BrowserContextPointerAnchor::new(target, position) {
            self.ui.browser_interaction.context_menu_pointer_anchor = Some(anchor);
        }
    }

    fn context_menu_pointer_position_for(
        &self,
        target: &BrowserContextPointerTarget,
        fallback: Point,
        current_pointer_position: Option<Point>,
    ) -> Point {
        current_pointer_position
            .filter(|position| position.x.is_finite() && position.y.is_finite())
            .or_else(|| {
                self.ui
                    .browser_interaction
                    .context_menu_pointer_anchor
                    .as_ref()
                    .filter(|anchor| &anchor.target == target)
                    .map(|anchor| anchor.position)
            })
            .unwrap_or(fallback)
    }
}

fn context_menu_title(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}
