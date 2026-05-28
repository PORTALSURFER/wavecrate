use super::{
    GuiAppState, GuiMessage, NormalizationProgress, NormalizationResult, NormalizedWaveformReload,
    WaveformPlaybackResume,
};
use crate::gui_app::{
    file_actions::{normalize_wav_file_in_place, sample_path_label},
    launch::emit_gui_action,
    waveform::WaveformState,
};
use std::time::Instant;
use wavecrate::audio::AudioPlayer;

impl GuiAppState {
    pub(super) fn focus_loaded_file(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if !self.waveform.has_loaded_sample() {
            self.sample_status = String::from("Load a sample to focus it");
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        }
        let path = self.waveform.path();
        if self.folder_browser.focus_file_across_sources(&path) {
            if let Some(index) = self.folder_browser.selected_audio_file_index() {
                context.scroll_fixed_row_into_view(
                    crate::gui_app::SAMPLE_BROWSER_LIST_ID,
                    index,
                    crate::gui_app::SAMPLE_BROWSER_ROW_HEIGHT,
                    crate::gui_app::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                    crate::gui_app::SAMPLE_BROWSER_EDGE_CONTEXT_ROWS,
                    0,
                );
            }
            self.sample_status = format!("Focused {}", sample_path_label(&path));
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "success",
                started_at,
                None,
            );
        } else {
            let error = format!(
                "Loaded sample is not visible in sources: {}",
                path.display()
            );
            self.sample_status = error.clone();
            emit_gui_action(
                "browser.focus_loaded_file",
                Some("browser"),
                None,
                "not_found",
                started_at,
                Some(&error),
            );
        }
    }

    pub(super) fn delete_selected_item(&mut self) {
        if self.folder_browser.selected_file_id().is_some() {
            self.delete_selected_files();
        } else {
            self.delete_selected_folder();
        }
    }

    fn delete_selected_folder(&mut self) {
        let started_at = Instant::now();
        let target = match self.folder_browser.selected_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        match self.folder_browser.delete_selected_folder() {
            Ok(status) => {
                self.sample_status = status;
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    Some(&target.name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "folder_browser.delete_selected",
                    Some("folder_browser"),
                    Some(&target.name),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn delete_selected_files(&mut self) {
        let started_at = Instant::now();
        let target = match self.folder_browser.selected_file_delete_target() {
            Ok(target) => target,
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    None,
                    "short_circuit",
                    started_at,
                    Some(&error),
                );
                return;
            }
        };
        let loaded_path = self.waveform.path();
        let deleting_loaded_sample = target.paths.iter().any(|path| path == &loaded_path);

        match self.folder_browser.delete_selected_files() {
            Ok(status) => {
                if deleting_loaded_sample {
                    if let Some(player) = self.audio_player.as_mut() {
                        player.stop();
                    }
                    self.waveform = WaveformState::empty();
                    self.current_playback_span = None;
                }
                self.sample_status = status;
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    Some(&target.label()),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "browser.delete_selected_files",
                    Some("browser"),
                    Some(&target.label()),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn extract_playmarked_range(&mut self) {
        let started_at = Instant::now();
        match self.waveform.extract_play_selection_to_sibling() {
            Ok(path) => {
                let label = sample_path_label(&path);
                self.waveform.flash_play_selection();
                self.folder_browser.refresh_file_path(&path);
                self.sample_status = format!("Extracted {label}");
                emit_gui_action(
                    "waveform.extract_playmarked_range",
                    Some("waveform"),
                    Some(&label),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.sample_status = error.clone();
                emit_gui_action(
                    "waveform.extract_playmarked_range",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn normalize_selected_samples(
        &mut self,
        context: &mut radiant::prelude::UpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        if self.normalization_progress.is_some() {
            self.sample_status = String::from("Normalization already in progress");
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "busy",
                started_at,
                None,
            );
            return;
        }
        let paths = self.folder_browser.selected_file_paths();
        if paths.is_empty() {
            self.sample_status = String::from("Select a sample to normalize");
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "empty",
                started_at,
                None,
            );
            return;
        }

        let loaded_path = self.waveform.path();
        let normalizing_loaded = paths.iter().any(|path| path == &loaded_path);
        let was_playing = self.waveform.is_playing() && normalizing_loaded;
        let restart_ratio = self
            .audio_player
            .as_ref()
            .and_then(AudioPlayer::progress)
            .or(self.waveform.playhead_ratio())
            .unwrap_or(0.0);
        let restart_span = self.current_playback_span;
        if was_playing {
            if let Some(player) = self.audio_player.as_mut() {
                player.stop();
            }
            self.waveform.stop_playback();
            self.current_playback_span = None;
        }

        let task_id = self.next_task_id;
        self.next_task_id = self.next_task_id.saturating_add(1);
        let label = normalize_progress_label(paths.len());
        self.normalization_progress = Some(NormalizationProgress {
            task_id,
            label: label.clone(),
            completed: 0,
            total: paths.len(),
            detail: String::from("Queued"),
        });
        self.sample_status = format!("Normalizing {label}");
        let sender = self.worker_sender.clone();
        let loaded_path_for_job = loaded_path.clone();
        context.spawn(
            "gui-normalize-selected-samples",
            move || {
                run_normalization_worker(
                    task_id,
                    paths,
                    loaded_path_for_job,
                    normalizing_loaded,
                    was_playing,
                    restart_ratio,
                    restart_span,
                    sender,
                )
            },
            GuiMessage::NormalizationFinished,
        );
        emit_gui_action(
            "browser.normalize_selected_samples",
            Some("browser"),
            None,
            "queued",
            started_at,
            None,
        );
    }

    pub(super) fn apply_normalization_progress(&mut self, progress: NormalizationProgress) {
        if self
            .normalization_progress
            .as_ref()
            .is_some_and(|active| active.task_id == progress.task_id)
        {
            self.normalization_progress = Some(progress);
        }
    }

    pub(super) fn finish_normalization(&mut self, result: NormalizationResult) {
        let started_at = Instant::now();
        if !self
            .normalization_progress
            .as_ref()
            .is_some_and(|active| active.task_id == result.task_id)
        {
            return;
        }
        self.normalization_progress = None;
        self.progress_tick = 0.0;

        for path in &result.normalized {
            self.folder_browser.refresh_file_path(path);
        }

        let mut last_error = result.last_error;
        if result.normalizing_loaded
            && result
                .normalized
                .iter()
                .any(|path| path == &result.loaded_path)
        {
            let playback = result.was_playing.then_some(WaveformPlaybackResume {
                start_ratio: result.restart_ratio,
                span: result.restart_span,
            });
            if let Err(error) = self.reload_normalized_waveform(NormalizedWaveformReload {
                path: &result.loaded_path,
                playback,
            }) {
                last_error = Some(error);
            }
        }

        let normalized = result.normalized;
        if let Some(error) = last_error {
            self.sample_status = format!(
                "Normalized {} sample{} | {error}",
                normalized.len(),
                if normalized.len() == 1 { "" } else { "s" }
            );
            emit_gui_action(
                "browser.normalize_selected_samples",
                Some("browser"),
                None,
                "partial_or_error",
                started_at,
                Some(&error),
            );
            return;
        }

        self.sample_status = match normalized.as_slice() {
            [] => String::from("No selected samples were normalized"),
            [path] => format!("Normalized {}", sample_path_label(path)),
            _ => format!("Normalized {} samples", normalized.len()),
        };
        emit_gui_action(
            "browser.normalize_selected_samples",
            Some("browser"),
            None,
            "success",
            started_at,
            None,
        );
    }
}

fn normalize_progress_label(count: usize) -> String {
    format!("{count} sample{}", if count == 1 { "" } else { "s" })
}

#[allow(clippy::too_many_arguments)]
fn run_normalization_worker(
    task_id: u64,
    paths: Vec<std::path::PathBuf>,
    loaded_path: std::path::PathBuf,
    normalizing_loaded: bool,
    was_playing: bool,
    restart_ratio: f32,
    restart_span: Option<(f32, f32)>,
    sender: std::sync::mpsc::Sender<GuiMessage>,
) -> NormalizationResult {
    let total = paths.len();
    let label = normalize_progress_label(total);
    let mut normalized = Vec::new();
    let mut last_error = None;
    for (index, path) in paths.iter().enumerate() {
        let detail = sample_path_label(path);
        let _ = sender.send(GuiMessage::NormalizationProgress(NormalizationProgress {
            task_id,
            label: label.clone(),
            completed: index,
            total,
            detail: detail.clone(),
        }));
        match normalize_wav_file_in_place(path) {
            Ok(()) => normalized.push(path.clone()),
            Err(error) => {
                last_error = Some(format!("{detail}: {error}"));
            }
        }
        let _ = sender.send(GuiMessage::NormalizationProgress(NormalizationProgress {
            task_id,
            label: label.clone(),
            completed: index + 1,
            total,
            detail,
        }));
    }
    NormalizationResult {
        task_id,
        loaded_path,
        normalizing_loaded,
        was_playing,
        restart_ratio,
        restart_span,
        normalized,
        last_error,
    }
}
