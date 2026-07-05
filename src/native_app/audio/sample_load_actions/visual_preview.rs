use radiant::prelude as ui;
use std::{path::Path, time::Instant};

use crate::native_app::{
    app::{GuiMessage, InstantWaveformPreviewResult, NativeAppState, sample_path_label},
    waveform::{
        InstantWaveformPreview, InstantWaveformPreviewTier, PreviewAuditionClip,
        instant_waveform_head_preview_from_clip, load_instant_waveform_coarse_preview,
    },
};

use super::{deferred_drop::defer_large_drop, log_sample_load_timing};

const INSTANT_WAVEFORM_PREVIEW_TASK_NAME: &str = "gui-instant-waveform-preview";

impl NativeAppState {
    pub(in crate::native_app) fn start_starmap_waveform_preview(
        &mut self,
        path: &str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
        started_at: Instant,
    ) {
        if self.waveform.current.has_loaded_sample()
            && self.waveform.current.path() == Path::new(path)
            && !self.waveform.instant_preview_active()
        {
            return;
        }
        if let Some(preview) = self
            .waveform
            .cache
            .instant_waveform_preview(Path::new(path))
        {
            let tier = preview.tier;
            self.replace_current_with_instant_waveform_preview(preview);
            if tier == InstantWaveformPreviewTier::Head {
                self.queue_instant_waveform_preview(path.to_owned(), None, started_at, context);
            }
            return;
        }
        if self.waveform.instant_preview_path() != Some(Path::new(path)) {
            let previous = self
                .waveform
                .replace_current_with_instant_waveform_preview_loading(
                    Path::new(path).to_path_buf(),
                );
            defer_large_drop(previous);
        }
        self.waveform.load.label = Some(sample_path_label(path));
        self.waveform.load.progress = 0.0;
        self.waveform.load.target_progress = 0.0;
        let clip = self.waveform.cache.preview_audition_clip(Path::new(path));
        self.queue_instant_waveform_preview(path.to_owned(), clip, started_at, context);
    }

    pub(in crate::native_app) fn finish_instant_waveform_preview(
        &mut self,
        completion: ui::TaskCompletion<InstantWaveformPreviewResult>,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(result) = self
            .background
            .instant_waveform_preview_task
            .finish_completion(completion)
        else {
            return;
        };
        if !self.instant_waveform_preview_matches_current_target(result.path.as_str()) {
            log_sample_load_timing(
                "browser.sample_load.instant_waveform_preview.stale",
                result.path.as_str(),
                started_at.elapsed(),
                false,
            );
            return;
        }
        let preview = match result.preview {
            Ok(preview) => preview,
            Err(error) => {
                log_sample_load_timing(
                    "browser.sample_load.instant_waveform_preview.error",
                    result.path.as_str(),
                    started_at.elapsed(),
                    false,
                );
                tracing::debug!(
                    target: "wavecrate::debug::sample_load",
                    path = %result.path,
                    error = %error,
                    "Instant waveform preview failed"
                );
                return;
            }
        };
        let tier = preview.tier;
        self.waveform
            .cache
            .store_instant_waveform_preview(preview.clone());
        self.replace_current_with_instant_waveform_preview(preview);
        self.waveform.load.label = None;
        context.request_paint_only();
        log_sample_load_timing(
            "browser.sample_load.instant_waveform_preview.ready",
            result.path.as_str(),
            started_at.elapsed(),
            false,
        );
        if tier == InstantWaveformPreviewTier::Head
            && self.instant_waveform_preview_matches_current_target(result.path.as_str())
        {
            self.queue_instant_waveform_preview(result.path, None, Instant::now(), context);
        }
    }

    fn queue_instant_waveform_preview(
        &mut self,
        path: String,
        clip: Option<PreviewAuditionClip>,
        started_at: Instant,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        context
            .business()
            .interactive(INSTANT_WAVEFORM_PREVIEW_TASK_NAME)
            .latest(&mut self.background.instant_waveform_preview_task)
            .run(
                move |worker_context| {
                    let preview = build_instant_waveform_preview(path.as_str(), clip, || {
                        worker_context.is_cancelled()
                    });
                    InstantWaveformPreviewResult { path, preview }
                },
                move |completion| GuiMessage::InstantWaveformPreviewFinished {
                    completion,
                    started_at,
                },
            );
    }

    fn replace_current_with_instant_waveform_preview(&mut self, preview: InstantWaveformPreview) {
        let previous = self
            .waveform
            .replace_current_with_instant_waveform_preview(preview);
        defer_large_drop(previous);
    }

    fn instant_waveform_preview_matches_current_target(&self, path: &str) -> bool {
        if self.ui.chrome.starmap_audition_drag.is_some()
            || self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .is_some()
        {
            return self
                .ui
                .chrome
                .starmap_audition_queue
                .active_file_id
                .as_deref()
                == Some(path);
        }
        self.library.folder_browser.selected_file_id() == Some(path)
    }
}

fn build_instant_waveform_preview(
    path: &str,
    clip: Option<PreviewAuditionClip>,
    cancelled: impl Fn() -> bool,
) -> Result<InstantWaveformPreview, String> {
    let progress = |_| {};
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    if let Some(clip) = clip {
        return instant_waveform_head_preview_from_clip(clip, &progress, &cancelled);
    }
    load_instant_waveform_coarse_preview(Path::new(path).to_path_buf(), &progress, &cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instant_waveform_preview_result_compares_path_and_errors_only() {
        let first = InstantWaveformPreviewResult {
            path: String::from("a.wav"),
            preview: Err(String::from("nope")),
        };
        let second = InstantWaveformPreviewResult {
            path: String::from("a.wav"),
            preview: Err(String::from("nope")),
        };

        assert_eq!(first, second);
    }
}
