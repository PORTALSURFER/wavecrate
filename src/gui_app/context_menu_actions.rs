use radiant::gui::types::Point;
use std::path::Path;
use std::time::Instant;

use super::context_menu::{self, BrowserContextMenu, BrowserContextTargetKind};
use super::file_actions::{format_copy_path, sample_path_label};
use super::{GuiAppState, emit_gui_action};

impl GuiAppState {
    pub(super) fn open_source_context_menu(&mut self, source_id: String, position: Point) {
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
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        let source_id = self
            .folder_browser
            .source_is_removable(&source_id)
            .then_some(source_id);
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Source,
            path,
            source_id,
            metadata_tag: None,
            anchor: position,
            title,
        });
    }

    pub(super) fn open_folder_context_menu(&mut self, folder_id: String, position: Point) {
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
        let title = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Folder,
            path,
            source_id: None,
            metadata_tag: None,
            anchor: position,
            title,
        });
    }

    pub(super) fn open_sample_context_menu(&mut self, path: String, position: Point) {
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
        let title = sample_path_label(&path);
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::Sample,
            path,
            source_id: None,
            metadata_tag: None,
            anchor: position,
            title,
        });
    }

    pub(super) fn open_metadata_tag_context_menu(&mut self, tag: String, position: Point) {
        self.context_menu = Some(BrowserContextMenu {
            kind: BrowserContextTargetKind::MetadataTag,
            path: Path::new("").to_path_buf(),
            source_id: None,
            metadata_tag: Some(tag.clone()),
            anchor: position,
            title: tag,
        });
    }

    pub(super) fn copy_context_path(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<super::GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.copy_path",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let path_text = format_copy_path(&menu.path);
        let kind = menu.kind;
        let path = menu.path;
        let completion_kind = kind.clone();
        let completion_path = path.clone();
        context.copy_text(path_text, move |result| {
            super::GuiMessage::ContextPathCopyFinished {
                kind: completion_kind,
                path: completion_path,
                result,
            }
        });
        emit_gui_action(
            "browser.context_menu.copy_path",
            Some(context_menu::pane(&kind)),
            None,
            "requested",
            started_at,
            None,
        );
    }

    pub(super) fn finish_context_path_copy(
        &mut self,
        kind: BrowserContextTargetKind,
        path: std::path::PathBuf,
        result: Result<radiant::prelude::PlatformResponse, String>,
    ) {
        let started_at = Instant::now();
        match completed_platform_action(result) {
            Ok(()) => {
                self.sample_status = String::from("Copied path");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = format!("Copy path failed: {error}");
                emit_gui_action(
                    "browser.context_menu.copy_path",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn open_context_target(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<super::GuiMessage>,
    ) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if !context_menu::target_available(&menu.kind, &menu.path) {
            let error = context_menu::missing_target_message(&menu.kind);
            self.sample_status = error.to_string();
            emit_gui_action(
                "browser.context_menu.open_explorer",
                Some(context_menu::pane(&menu.kind)),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some(error),
            );
            return;
        }
        let kind = menu.kind;
        let path = menu.path;
        match kind {
            BrowserContextTargetKind::Source | BrowserContextTargetKind::Folder => {
                let completion_kind = kind.clone();
                let completion_path = path.clone();
                context.open_path(path.clone(), move |result| {
                    super::GuiMessage::ContextTargetOpenFinished {
                        kind: completion_kind,
                        path: completion_path,
                        result,
                    }
                });
            }
            BrowserContextTargetKind::Sample => {
                let completion_kind = kind.clone();
                let completion_path = path.clone();
                context.reveal_path(path.clone(), move |result| {
                    super::GuiMessage::ContextTargetOpenFinished {
                        kind: completion_kind,
                        path: completion_path,
                        result,
                    }
                });
            }
            BrowserContextTargetKind::MetadataTag => return,
        };
        emit_gui_action(
            "browser.context_menu.open_explorer",
            Some(context_menu::pane(&kind)),
            Some(context_menu::target_label(&path).as_str()),
            "requested",
            started_at,
            None,
        );
    }

    pub(super) fn finish_context_target_open(
        &mut self,
        kind: BrowserContextTargetKind,
        path: std::path::PathBuf,
        result: Result<radiant::prelude::PlatformResponse, String>,
    ) {
        let started_at = Instant::now();
        match completed_platform_action(result) {
            Ok(()) => {
                self.sample_status = match &kind {
                    BrowserContextTargetKind::Sample => String::from("Revealed sample"),
                    BrowserContextTargetKind::Source => String::from("Opened source folder"),
                    BrowserContextTargetKind::Folder => String::from("Opened folder"),
                    BrowserContextTargetKind::MetadataTag => String::from("Tag action complete"),
                };
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.context_menu.open_explorer",
                    Some(context_menu::pane(&kind)),
                    Some(context_menu::target_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn remove_context_source(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::Source {
            self.sample_status = String::from("Context target is not a source");
            emit_gui_action(
                "browser.context_menu.source.remove",
                Some("sources"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "error",
                started_at,
                Some("target is not a source"),
            );
            return;
        }
        let Some(source_id) = menu.source_id else {
            self.sample_status = String::from("Default source cannot be removed");
            emit_gui_action(
                "browser.context_menu.source.remove",
                Some("sources"),
                Some(context_menu::target_label(&menu.path).as_str()),
                "blocked",
                started_at,
                Some("source not removable"),
            );
            return;
        };
        let loaded_path = self.waveform.path();
        match self.folder_browser.remove_source(&source_id) {
            Ok(removed) => {
                if path_is_within(&loaded_path, &removed.root) {
                    if let Some(player) = self.audio_player.as_mut() {
                        player.stop();
                    }
                    self.waveform = super::WaveformState::empty();
                    self.current_playback_span = None;
                }
                self.sample_status = format!("Removed source {}", removed.label);
                self.persist_user_configuration("folder_browser.source.remove.persist", started_at);
                emit_gui_action(
                    "browser.context_menu.source.remove",
                    Some("sources"),
                    Some(&removed.label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.context_menu.source.remove",
                    Some("sources"),
                    Some(context_menu::target_label(&menu.path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn delete_context_metadata_tag(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<super::GuiMessage>,
    ) {
        let Some(menu) = self.context_menu.take() else {
            return;
        };
        if menu.kind != BrowserContextTargetKind::MetadataTag {
            self.sample_status = String::from("Context target is not a tag");
            return;
        }
        let Some(tag) = menu.metadata_tag else {
            self.sample_status = String::from("Tag is unavailable");
            return;
        };
        self.delete_metadata_tag_from_library(tag, context);
    }
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    !path.as_os_str().is_empty() && path.starts_with(root)
}

fn completed_platform_action(
    result: Result<radiant::prelude::PlatformResponse, String>,
) -> Result<(), String> {
    result?
        .into_completed()
        .map_err(|other| format!("unexpected platform response: {other:?}"))
}
