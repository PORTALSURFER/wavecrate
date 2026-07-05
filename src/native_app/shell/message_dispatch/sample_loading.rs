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
            GuiMessage::SettledSamplePromotion {
                ticket,
                path,
                scheduled_at,
            } => self.promote_settled_sample_to_full_playback(ticket, path, scheduled_at, context),
            GuiMessage::SampleLoadPathValidated {
                completion,
                started_at,
            } => self.finish_sample_load_path_validation(completion, started_at, context),
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
            GuiMessage::PreviewAuditionDecoded {
                completion,
                started_at,
            } => self.finish_preview_audition_decode(completion, started_at, context),
            GuiMessage::PreviewAuditionWarmFinished {
                completion,
                started_at,
            } => self.finish_preview_audition_warm(completion, started_at),
            GuiMessage::InstantWaveformPreviewFinished {
                completion,
                started_at,
            } => self.finish_instant_waveform_preview(completion, started_at, context),
            GuiMessage::SampleLoadFinished(result) => self.finish_sample_load(result, context),
            GuiMessage::WaveformCacheIndicatorRefreshFinished(result) => {
                self.finish_waveform_cache_indicator_refresh(result)
            }
            GuiMessage::WaveformCacheWarmFinished(result) => {
                self.finish_waveform_cache_warm(result)
            }
            GuiMessage::ActiveFolderCacheWarmPlanProgress(progress) => {
                self.apply_active_folder_cache_warm_plan_progress(progress);
            }
            GuiMessage::ActiveFolderCacheWarmPlanned(result) => {
                self.finish_active_folder_cache_warm_plan(result, context);
            }
            GuiMessage::ActiveFolderCacheWarmReady(ticket) => {
                self.start_active_folder_cache_warm_after_delay(ticket, context);
            }
            GuiMessage::ActiveFolderCacheWarmProgress(progress) => {
                self.apply_active_folder_cache_warm_progress(progress);
            }
            GuiMessage::ActiveFolderCacheWarmFinished(result) => {
                self.finish_active_folder_cache_warm(result, context);
            }
            _ => unreachable!("sample-loading dispatcher received a non-sample-loading message"),
        }
    }
}
