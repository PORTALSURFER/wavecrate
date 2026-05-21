use super::*;
use span::ResolvedPlaybackSpan;

mod loop_control;
mod progress;
mod span;

impl GuiAppState {
    pub(super) fn play_selected_sample(&mut self, context: &mut ui::UpdateContext<GuiMessage>) {
        let started_at = Instant::now();
        if let Some(path) = self.folder_browser.selected_file_id()
            && PathBuf::from(path) != self.waveform.path()
        {
            let label = sample_path_label(path);
            emit_gui_action(
                "playback.play_selected_sample",
                Some("transport"),
                Some(&label),
                "load_queued",
                started_at,
                None,
            );
            self.select_sample(path.to_string(), context);
            return;
        }
        let (start, end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        match self.start_playback_current_span(start, end) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status = format!("Playing {file_name}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_selected_sample",
                    Some("transport"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn play_waveform_from_ratio(&mut self, start_ratio: f32) {
        let started_at = Instant::now();
        match self.start_playback_current_span(start_ratio, 1.0) {
            Ok(()) => {
                let file_name = self.waveform.file_name();
                self.sample_status =
                    format!("Playing {} from {:.1}%", file_name, start_ratio * 100.0);
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    Some(&file_name),
                    "success",
                    started_at,
                    None,
                );
            }
            Err(err) => {
                self.sample_status = format!("Playback unavailable: {err}");
                emit_gui_action(
                    "playback.play_waveform_from_ratio",
                    Some("waveform"),
                    None,
                    "error",
                    started_at,
                    Some(&err),
                );
            }
        }
    }

    pub(super) fn stop_playback(&mut self) {
        let started_at = Instant::now();
        if let Some(player) = self.audio_player.as_mut() {
            player.stop();
        }
        self.waveform.stop_playback();
        self.current_playback_span = None;
        let file_name = self.waveform.file_name();
        self.sample_status = format!("Stopped {file_name}");
        emit_gui_action(
            "playback.stop",
            Some("transport"),
            Some(&file_name),
            "success",
            started_at,
            None,
        );
    }

    pub(super) fn start_playback_current_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
    ) -> Result<(), String> {
        self.start_playback_span(start_ratio, end_ratio, None)
    }

    pub(super) fn start_playback_span(
        &mut self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> Result<(), String> {
        if self.audio_player.is_none() {
            self.open_configured_audio_player()?;
        }
        if !self.waveform.has_loaded_sample() {
            return Err(String::from("Select a sample to load"));
        }
        let playback_span = self.resolve_playback_span(start_ratio, end_ratio, loop_offset_ratio);
        let start_ratio = playback_span.start_ratio;
        let end_ratio = playback_span.end_ratio;
        let duration = self.waveform.frames() as f32 / self.waveform.sample_rate().max(1) as f32;
        let player = self
            .audio_player
            .as_mut()
            .ok_or_else(|| String::from("audio player did not initialize"))?;
        player.set_volume(self.volume);
        self.audio_output_resolved = Some(player.output_details().clone());
        player.set_audio(self.waveform.audio_bytes(), duration);
        player.set_edit_fade_state(self.waveform.edit_selection());
        let playback_start = if self.loop_playback {
            player.play_looped_range_from(
                f64::from(start_ratio),
                f64::from(end_ratio),
                f64::from(playback_span.offset_ratio),
            )?;
            playback_span.offset_ratio
        } else {
            player.play_range(f64::from(start_ratio), f64::from(end_ratio), false)?;
            start_ratio
        };
        self.waveform.start_playback(playback_start);
        self.current_playback_span = Some((start_ratio, end_ratio));
        Ok(())
    }

    pub(super) fn resolve_playback_span(
        &self,
        start_ratio: f32,
        end_ratio: f32,
        loop_offset_ratio: Option<f32>,
    ) -> ResolvedPlaybackSpan {
        let requested_start = start_ratio.clamp(0.0, 1.0);
        let requested_end = end_ratio.clamp(requested_start, 1.0);
        if !self.loop_playback {
            return ResolvedPlaybackSpan {
                start_ratio: requested_start,
                end_ratio: requested_end,
                offset_ratio: requested_start,
            };
        }

        let (loop_start, loop_end) = self
            .waveform
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        let start_ratio = loop_start.clamp(0.0, 1.0);
        let end_ratio = loop_end.clamp(start_ratio, 1.0);
        let requested_offset = loop_offset_ratio.unwrap_or(requested_start).clamp(0.0, 1.0);
        let offset_ratio = if (start_ratio..=end_ratio).contains(&requested_offset) {
            requested_offset
        } else {
            start_ratio
        };

        ResolvedPlaybackSpan {
            start_ratio,
            end_ratio,
            offset_ratio,
        }
    }
}
