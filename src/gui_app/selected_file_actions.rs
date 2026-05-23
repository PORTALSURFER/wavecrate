use super::{GuiAppState, NormalizedWaveformReload, WaveformPlaybackResume};
use crate::gui_app::{
    file_actions::{normalize_wav_file_in_place, sample_path_label},
    launch::emit_gui_action,
    waveform::WaveformState,
};
use std::time::Instant;
use wavecrate::audio::AudioPlayer;

impl GuiAppState {
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

    pub(super) fn normalize_selected_samples(&mut self) {
        let started_at = Instant::now();
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

        let mut normalized = Vec::new();
        let mut last_error = None;
        for path in &paths {
            match normalize_wav_file_in_place(path) {
                Ok(()) => {
                    self.folder_browser.refresh_file_path(path);
                    normalized.push(path.clone());
                }
                Err(error) => {
                    last_error = Some(format!("{}: {error}", sample_path_label(path)));
                }
            }
        }

        if normalizing_loaded && normalized.iter().any(|path| path == &loaded_path) {
            let playback = was_playing.then_some(WaveformPlaybackResume {
                start_ratio: restart_ratio,
                span: restart_span,
            });
            if let Err(error) = self.reload_normalized_waveform(NormalizedWaveformReload {
                path: &loaded_path,
                playback,
            }) {
                last_error = Some(error);
            }
        }

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
