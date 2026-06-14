use radiant::prelude as ui;
use std::time::Instant;

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
        let completion = SampleLoadCompletion::from_task(
            load,
            self.background.sample_load_tasks.finish_key(&key, ticket),
        );
        match completion {
            SampleLoadCompletion::Stale { label } => {
                self.audio.pending_sample_playback = None;
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
                self.clear_sample_loading_state();
                self.waveform
                    .load
                    .selection
                    .failed(path.as_str(), error.clone());
                self.audio.pending_sample_playback = None;
                self.ui.status.sample = format!("Could not load sample: {error}");
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
            } => self.finish_loaded_sample_load(path, *waveform, autoplay, started_at, context),
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
