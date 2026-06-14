use radiant::prelude as ui;

use crate::native_app::app::{GuiMessage, NativeAppState};

impl NativeAppState {
    pub(super) fn apply_sample_loading_dispatch(
        &mut self,
        message: GuiMessage,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        match message {
            GuiMessage::DeferredSampleLoad {
                ticket,
                path,
                autoplay,
                check_cache,
                scheduled_at,
            } => {
                self.start_deferred_sample_load(
                    ticket,
                    path,
                    autoplay,
                    check_cache,
                    scheduled_at,
                    context,
                );
            }
            GuiMessage::SampleLoadProgress(key, ticket, progress) => {
                if self
                    .background
                    .sample_load_tasks
                    .is_active_key(&key, ticket)
                {
                    self.waveform.load.target_progress = progress.clamp(0.0, 0.995);
                }
            }
            GuiMessage::SamplePlaybackReady(result) => {
                self.finish_sample_playback_ready(result, context)
            }
            GuiMessage::SampleLoadFinished(result) => self.finish_sample_load(result, context),
            GuiMessage::WaveformCacheIndicatorRefreshFinished(result) => {
                self.finish_waveform_cache_indicator_refresh(result)
            }
            GuiMessage::WaveformCacheWarmFinished(result) => {
                self.finish_waveform_cache_warm(result)
            }
            GuiMessage::ActiveFolderCacheWarmReady(ticket) => {
                self.start_active_folder_cache_warm_after_delay(ticket, context);
            }
            GuiMessage::ActiveFolderCacheWarmFinished(result) => {
                self.finish_active_folder_cache_warm(result, context);
            }
            _ => unreachable!("sample-loading dispatcher received a non-sample-loading message"),
        }
    }
}
