use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;
use wavecrate_library::sample_sources::is_supported_audio;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};

impl NativeAppState {
    pub(in crate::native_app) fn open_audio_documents(
        &mut self,
        paths: Vec<PathBuf>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        for path in paths {
            self.open_audio_document(path, context, started_at);
        }
    }

    fn open_audio_document(
        &mut self,
        path: PathBuf,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        if !path.is_file() {
            let error = format!(
                "Open audio failed: {} is not a file",
                sample_path_label(&path)
            );
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "waveform.native_file_open",
                Some("waveform"),
                Some(sample_path_label(&path).as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return;
        }
        if !is_supported_audio(&path) {
            self.ui.status.sample =
                format!("Unsupported audio document: {}", sample_path_label(&path));
            emit_gui_action(
                "waveform.native_file_open",
                Some("waveform"),
                Some(sample_path_label(&path).as_str()),
                "unsupported",
                started_at,
                Some("unsupported file type"),
            );
            return;
        }
        if self
            .library
            .folder_browser
            .sample_source_for_file_path(&path)
            .is_some()
        {
            if self.focus_and_load_audio_document(&path, context, started_at) {
                return;
            }
            self.queue_configured_source_document_open(path, context, started_at);
            return;
        }
        self.queue_parent_source_document_open(path, context, started_at);
    }

    fn queue_configured_source_document_open(
        &mut self,
        path: PathBuf,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        let Some((source, _)) = self
            .library
            .folder_browser
            .sample_source_for_file_path(&path)
        else {
            return;
        };
        self.library.queue_pending_audio_document_open(path.clone());
        let task_id = self.next_folder_task_id();
        if let Some(request) = self
            .library
            .begin_source_scan(source.id.as_str().to_string(), task_id)
        {
            emit_gui_action(
                "waveform.native_file_open",
                Some("waveform"),
                Some(sample_path_label(&path).as_str()),
                "source_scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            self.ui.status.sample = format!(
                "Waiting for source scan to open {}",
                sample_path_label(&path)
            );
        }
    }

    fn queue_parent_source_document_open(
        &mut self,
        path: PathBuf,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        let Some(parent) = path.parent().map(Path::to_path_buf) else {
            let error = format!("Open audio failed: {} has no parent folder", path.display());
            self.ui.status.sample = error.clone();
            emit_gui_action(
                "waveform.native_file_open",
                Some("waveform"),
                Some(sample_path_label(&path).as_str()),
                "error",
                started_at,
                Some(&error),
            );
            return;
        };
        self.library.queue_pending_audio_document_open(path.clone());
        let task_id = self.next_folder_task_id();
        if let Some(request) = self.library.begin_add_source_path(parent, task_id) {
            emit_gui_action(
                "waveform.native_file_open",
                Some("waveform"),
                Some(sample_path_label(&path).as_str()),
                "source_scan_queued",
                started_at,
                None,
            );
            self.launch_folder_scan(request, context);
        } else {
            self.open_ready_audio_documents(context, started_at);
        }
    }

    pub(in crate::native_app) fn open_ready_audio_documents(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        let pending = self.library.take_pending_audio_document_opens();
        if pending.is_empty() {
            return;
        }
        let mut remaining = Vec::new();
        for path in pending {
            if !self.focus_and_load_audio_document(&path, context, started_at) {
                remaining.push(path);
            }
        }
        self.library.restore_pending_audio_document_opens(remaining);
    }

    fn focus_and_load_audio_document(
        &mut self,
        path: &Path,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) -> bool {
        self.library
            .folder_browser
            .refresh_file_path_across_sources(path);
        if !self.library.folder_browser.focus_file_across_sources(path) {
            return false;
        }
        let file_id = path.display().to_string();
        self.audio.pending_sample_playback = None;
        self.load_sample_without_autoplay(file_id, context);
        self.ui.status.sample = format!("Opened {}", sample_path_label(path));
        emit_gui_action(
            "waveform.native_file_open",
            Some("waveform"),
            Some(sample_path_label(path).as_str()),
            "opened",
            started_at,
            None,
        );
        true
    }
}
