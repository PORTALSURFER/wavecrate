use radiant::prelude as ui;
use radiant::prelude::PlatformResultExt as _;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use crate::native_app::app::{
    GuiMessage, NativeAppState, WaveformState, emit_gui_action, sample_path_label,
};
use crate::native_app::sample_library::context_menu_target::BrowserContextTargetKind;

impl NativeAppState {
    pub(in crate::native_app) fn pick_trash_folder(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        context.pick_folder(
            ui::FileDialogRequest::new().title("Choose trash folder"),
            GuiMessage::TrashFolderDialogFinished,
        );
        emit_gui_action(
            "settings.trash_folder.pick",
            Some("settings"),
            None,
            "requested",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn finish_trash_folder_dialog(&mut self, result: ui::PlatformResult) {
        let started_at = Instant::now();
        let path = match result.into_path_or_canceled() {
            Ok(Some(path)) => path,
            Ok(None) => {
                emit_gui_action(
                    "settings.trash_folder.pick",
                    Some("settings"),
                    None,
                    "cancelled",
                    started_at,
                    None,
                );
                return;
            }
            Err(error) => {
                self.sample_status = format!("Trash folder selection failed: {error}");
                emit_gui_action(
                    "settings.trash_folder.pick",
                    Some("settings"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        self.persisted_settings.trash_folder = Some(path.clone());
        self.persist_user_configuration("settings.trash_folder.persist", started_at);
        self.sample_status = format!("Trash folder set to {}", path.display());
        let target = path.display().to_string();
        emit_gui_action(
            "settings.trash_folder.pick",
            Some("settings"),
            Some(target.as_str()),
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn clear_trash_folder(&mut self) {
        let started_at = Instant::now();
        self.persisted_settings.trash_folder = None;
        self.persist_user_configuration("settings.trash_folder.clear", started_at);
        self.sample_status = String::from("Trash folder cleared");
        emit_gui_action(
            "settings.trash_folder.clear",
            Some("settings"),
            None,
            "success",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn move_context_target_to_trash(&mut self) {
        let started_at = Instant::now();
        let Some(menu) = self.browser_interaction.context_menu.take() else {
            return;
        };
        match menu.kind {
            BrowserContextTargetKind::Folder => {
                self.move_folder_path_to_trash(
                    menu.path,
                    "browser.context_menu.folder.trash",
                    started_at,
                );
            }
            BrowserContextTargetKind::Sample => {
                self.move_file_paths_to_trash(
                    vec![menu.path],
                    "browser.context_menu.sample.trash",
                    started_at,
                );
            }
            BrowserContextTargetKind::Source | BrowserContextTargetKind::MetadataTag => {
                self.sample_status = String::from("Context target cannot be moved to trash");
                emit_gui_action(
                    "browser.context_menu.trash",
                    Some("browser"),
                    None,
                    "blocked",
                    started_at,
                    Some("unsupported target"),
                );
            }
        }
    }

    pub(in crate::native_app) fn move_selected_folder_to_trash(
        &mut self,
        path: PathBuf,
        started_at: Instant,
    ) {
        self.move_folder_path_to_trash(path, "folder_browser.delete_selected", started_at);
    }

    pub(in crate::native_app) fn move_selected_files_to_trash(
        &mut self,
        paths: Vec<PathBuf>,
        started_at: Instant,
    ) {
        self.move_file_paths_to_trash(paths, "browser.delete_selected_files", started_at);
    }

    fn move_folder_path_to_trash(
        &mut self,
        path: PathBuf,
        action: &'static str,
        started_at: Instant,
    ) {
        match move_path_to_configured_trash(&path, self.persisted_settings.trash_folder.as_deref())
        {
            Ok(destination) => {
                self.folder_browser.discard_trashed_folder_path(&path);
                self.clear_loaded_sample_if_path_within(&path);
                self.sample_status = format!("Moved {} to trash", sample_path_label(&destination));
                emit_gui_action(
                    action,
                    Some("folder_browser"),
                    Some(sample_path_label(&path).as_str()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    action,
                    Some("folder_browser"),
                    Some(sample_path_label(&path).as_str()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn move_file_paths_to_trash(
        &mut self,
        paths: Vec<PathBuf>,
        action: &'static str,
        started_at: Instant,
    ) {
        match move_paths_to_configured_trash(
            &paths,
            self.persisted_settings.trash_folder.as_deref(),
        ) {
            Ok(moved) => {
                self.folder_browser.discard_trashed_file_paths(&paths);
                for path in &paths {
                    self.clear_loaded_sample_if_exact(path);
                }
                let count = moved.len();
                let noun = if count == 1 { "file" } else { "files" };
                self.sample_status = format!("Moved {count} {noun} to trash");
                emit_gui_action(
                    action,
                    Some("browser"),
                    Some(&format!("{count} {noun}")),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    action,
                    Some("browser"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn clear_loaded_sample_if_exact(&mut self, path: &Path) {
        if self.waveform.path() == path {
            if let Some(player) = self.audio.player.as_mut() {
                player.stop();
            }
            self.waveform = WaveformState::empty();
            self.audio.current_playback_span = None;
        }
    }

    fn clear_loaded_sample_if_path_within(&mut self, root: &Path) {
        let loaded_path = self.waveform.path();
        if !loaded_path.as_os_str().is_empty() && loaded_path.starts_with(root) {
            if let Some(player) = self.audio.player.as_mut() {
                player.stop();
            }
            self.waveform = WaveformState::empty();
            self.audio.current_playback_span = None;
        }
    }
}

fn move_paths_to_configured_trash(
    paths: &[PathBuf],
    trash_folder: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let mut moved = Vec::with_capacity(paths.len());
    for path in paths {
        moved.push(move_path_to_configured_trash(path, trash_folder)?);
    }
    Ok(moved)
}

fn move_path_to_configured_trash(
    path: &Path,
    trash_folder: Option<&Path>,
) -> Result<PathBuf, String> {
    let trash_folder = trash_folder.ok_or_else(|| {
        String::from("Set a trash folder in Settings > General before deleting files")
    })?;
    if !path.exists() {
        return Err(format!("Trash move failed: {} is missing", path.display()));
    }
    fs::create_dir_all(trash_folder).map_err(|err| format!("Create trash folder failed: {err}"))?;
    let trash_folder = trash_folder
        .canonicalize()
        .map_err(|err| format!("Trash folder is unavailable: {err}"))?;
    let source = path
        .canonicalize()
        .map_err(|err| format!("Trash source is unavailable: {err}"))?;
    if source.starts_with(&trash_folder) {
        return Err(String::from(
            "Selected item is already inside the trash folder",
        ));
    }
    if trash_folder.starts_with(&source) {
        return Err(String::from(
            "Trash folder cannot be inside the item being deleted",
        ));
    }
    let destination = next_available_trash_path(&trash_folder, &source)?;
    move_path(&source, &destination)?;
    Ok(destination)
}

fn next_available_trash_path(trash_folder: &Path, source: &Path) -> Result<PathBuf, String> {
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("Trash move failed: {} has no file name", source.display()))?;
    let candidate = trash_folder.join(file_name);
    if !candidate.exists() {
        return Ok(candidate);
    }
    let stem = source
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string_lossy().to_string());
    let extension = source.extension().map(|extension| extension.to_os_string());
    for index in 2..10_000 {
        let mut name = format!("{stem} {index}");
        if let Some(extension) = &extension {
            name.push('.');
            name.push_str(&extension.to_string_lossy());
        }
        let candidate = trash_folder.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(String::from(
        "Trash folder contains too many matching names",
    ))
}

fn move_path(source: &Path, destination: &Path) -> Result<(), String> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            if source.is_dir() {
                copy_dir_all(source, destination)
                    .and_then(|()| fs::remove_dir_all(source))
                    .map_err(|err| {
                        format!(
                            "Move folder to trash failed: {rename_error}; fallback failed: {err}"
                        )
                    })
            } else {
                fs::copy(source, destination)
                    .and_then(|_| fs::remove_file(source))
                    .map_err(|err| {
                        format!("Move file to trash failed: {rename_error}; fallback failed: {err}")
                    })
            }
        }
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
