use radiant::prelude as ui;
use std::{path::Path, time::Instant};

use crate::native_app::app::{
    NativeAppState, SampleLoadResult, SampleLoadTaskCompletion, WaveformState, emit_gui_action,
    sample_path_label,
};

pub(super) enum SampleLoadCompletion {
    Stale {
        label: String,
    },
    Loaded {
        path: String,
        waveform: Box<WaveformState>,
        autoplay: bool,
        display_after_instant_audition: bool,
    },
    Failed {
        path: String,
        label: String,
        error: String,
    },
}

impl SampleLoadCompletion {
    pub(super) fn from_task(
        completion: SampleLoadTaskCompletion<SampleLoadResult>,
        task_is_current: bool,
    ) -> Self {
        let load = completion.output;
        let label = sample_path_label(load.path.as_str());
        if !task_is_current {
            return Self::Stale { label };
        }
        match load.result {
            Ok(waveform) => Self::Loaded {
                path: load.path,
                waveform: Box::new(waveform),
                autoplay: load.autoplay,
                display_after_instant_audition: load.display_after_instant_audition,
            },
            Err(error) => Self::Failed {
                path: load.path,
                label,
                error,
            },
        }
    }
}

impl NativeAppState {
    pub(in crate::native_app) fn finish_sample_load(
        &mut self,
        load: SampleLoadTaskCompletion<SampleLoadResult>,
        context: &mut ui::UiUpdateContext<crate::native_app::app::GuiMessage>,
    ) {
        let started_at = Instant::now();
        let ticket = load.ticket;
        let key = load.key.clone();
        let task_was_current = self.background.sample_load_tasks.finish_key(&key, ticket);
        let replacement_is_active = self.background.sample_load_tasks.active(&key).is_some();
        let completion = SampleLoadCompletion::from_task(load, task_was_current);
        match completion {
            SampleLoadCompletion::Stale { label } => {
                if !replacement_is_active {
                    self.audio.pending_sample_playback = None;
                }
                self.log_sample_identity_checkpoint(
                    "browser.sample_load.finish_stale",
                    "finish_sample_load",
                    None,
                    Some(label.as_str()),
                );
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "stale",
                    started_at,
                    None,
                );
            }
            SampleLoadCompletion::Failed { path, label, error } => {
                if self.audio.active_sample_playback_matches(path.as_str()) {
                    self.stop_current_sample_playback_for_load();
                    self.audio.clear_playback_progress();
                }
                self.clear_failed_playback_visual_handoff(Path::new(&path));
                self.clear_sample_loading_state();
                self.waveform
                    .load
                    .selection
                    .failed(path.as_str(), error.clone());
                self.audio.pending_sample_playback = None;
                self.focus_browser_file_for_playback_navigation(Path::new(&path), context);
                self.ui.status.sample = format!("Could not load {label}: {error}");
                self.ui.chrome.starmap_audition_queue.active_file_id = None;
                self.start_next_starmap_audition_hit(context);
                self.log_sample_identity_checkpoint(
                    "browser.sample_load.finish_failed",
                    "finish_sample_load",
                    Some(Path::new(&path)),
                    Some(error.as_str()),
                );
                emit_gui_action(
                    "browser.sample_load.finish",
                    Some("browser"),
                    Some(&label),
                    "error",
                    started_at,
                    Some(&error),
                );
            }
            SampleLoadCompletion::Loaded {
                path,
                waveform,
                autoplay,
                display_after_instant_audition,
            } => {
                if self.waveform.load.selection.selected_path.as_deref() != Some(path.as_str()) {
                    self.audio.pending_sample_playback = None;
                    self.log_sample_identity_checkpoint(
                        "browser.sample_load.finish_unexpected_path",
                        "finish_sample_load",
                        Some(Path::new(&path)),
                        self.waveform.load.selection.selected_path.as_deref(),
                    );
                    emit_gui_action(
                        "browser.sample_load.finish",
                        Some("browser"),
                        Some(&sample_path_label(path.as_str())),
                        "stale_selection",
                        started_at,
                        None,
                    );
                    return;
                }
                self.finish_loaded_sample_load(
                    path,
                    *waveform,
                    autoplay,
                    display_after_instant_audition,
                    started_at,
                    context,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn completion(
        path: &str,
        result: Result<WaveformState, String>,
    ) -> SampleLoadTaskCompletion<SampleLoadResult> {
        let mut latest = ui::LatestTask::new();
        ui::KeyedTaskCompletion {
            key: crate::native_app::audio::sample_load_actions::sample_resource_key(path),
            ticket: latest.begin(),
            output: SampleLoadResult {
                path: String::from(path),
                result,
                autoplay: true,
                display_after_instant_audition: false,
            },
        }
    }

    #[test]
    fn stale_completion_ignores_worker_error() {
        let completion = SampleLoadCompletion::from_task(
            completion("C:/samples/kick.wav", Err(String::from("decode failed"))),
            false,
        );

        assert!(matches!(completion, SampleLoadCompletion::Stale { .. }));
    }

    #[test]
    fn failed_completion_preserves_error() {
        let completion = SampleLoadCompletion::from_task(
            completion("C:/samples/kick.wav", Err(String::from("decode failed"))),
            true,
        );

        let SampleLoadCompletion::Failed { error, .. } = completion else {
            panic!("expected failed sample-load completion");
        };
        assert_eq!(error, "decode failed");
    }
}
