use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState, emit_gui_action, sample_path_label};

mod validation_worker;

const NATIVE_AUDIO_DOCUMENT_OPEN_VALIDATION_TASK_NAME: &str = "gui-native-file-open-validate";

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct NativeAudioDocumentOpenValidation {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) result: Result<(), NativeAudioDocumentOpenRejection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) enum NativeAudioDocumentOpenRejection {
    NotFile { message: String },
    Unsupported { message: String },
}

impl NativeAudioDocumentOpenRejection {
    fn action(&self) -> &'static str {
        match self {
            Self::NotFile { .. } => "error",
            Self::Unsupported { .. } => "unsupported",
        }
    }

    fn detail(&self) -> &'static str {
        match self {
            Self::NotFile { .. } => "not a file",
            Self::Unsupported { .. } => "unsupported file type",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::NotFile { message } | Self::Unsupported { message } => message,
        }
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn open_audio_documents(
        &mut self,
        paths: Vec<PathBuf>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        for path in paths {
            self.queue_audio_document_open_validation(path, context, started_at);
        }
    }

    fn queue_audio_document_open_validation(
        &mut self,
        path: PathBuf,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        context
            .business()
            .interactive(NATIVE_AUDIO_DOCUMENT_OPEN_VALIDATION_TASK_NAME)
            .run(
                move |_| validation_worker::validate_audio_document_open(path),
                move |validation| GuiMessage::NativeAudioDocumentOpenValidated {
                    started_at,
                    validation,
                },
            );
    }

    pub(in crate::native_app) fn finish_audio_document_open_validation(
        &mut self,
        started_at: Instant,
        validation: NativeAudioDocumentOpenValidation,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match validation.result {
            Ok(()) => self.open_validated_audio_document(validation.path, context, started_at),
            Err(rejection) => {
                let message = rejection.message().to_string();
                self.ui.status.sample = message.clone();
                emit_gui_action(
                    "waveform.native_file_open",
                    Some("waveform"),
                    Some(sample_path_label(&validation.path).as_str()),
                    rejection.action(),
                    started_at,
                    Some(rejection.detail()),
                );
            }
        }
    }

    fn open_validated_audio_document(
        &mut self,
        path: PathBuf,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
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
            let source_loaded = self
                .library
                .folder_browser
                .sample_source_for_file_path(&path)
                .is_some_and(|(source, _)| {
                    self.library
                        .folder_browser
                        .source_tree_loaded(source.id.as_str())
                });
            if !source_loaded {
                remaining.push(path);
                continue;
            }
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
        self.load_validated_sample_without_autoplay(file_id, context, started_at);
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
