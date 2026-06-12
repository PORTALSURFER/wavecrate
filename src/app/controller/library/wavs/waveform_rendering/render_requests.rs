use super::*;
use crate::app::controller::jobs::{JobMessage, WaveformRenderJob, WaveformRenderKey};
use crate::app::controller::state::runtime::{DerivedNodeId, DirtyReason, PendingWaveformRender};
use crate::waveform::DecodedWaveform;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
struct WaveformRenderRequest {
    decoded: Arc<DecodedWaveform>,
    meta: WaveformRenderMeta,
    view: WaveformView,
    effective_width: u32,
}

impl AppController {
    /// Queue or perform waveform raster generation for the current view.
    pub(super) fn refresh_waveform_image_now(&mut self) {
        if waveform_render_async_enabled() {
            self.queue_waveform_render_now();
            return;
        }
        self.refresh_waveform_image_now_sync();
    }

    /// Render waveform pixels for the current view immediately.
    fn refresh_waveform_image_now_sync(&mut self) {
        let Some(request) = self.plan_waveform_render_request() else {
            return;
        };

        if self
            .sample_view
            .waveform
            .render_meta
            .as_ref()
            .is_some_and(|meta: &WaveformRenderMeta| meta.matches(&request.meta))
        {
            return;
        }
        if let (Some(previous_meta), Some(previous_image)) = (
            self.sample_view.waveform.render_meta.as_ref(),
            self.ui.waveform.image.as_ref(),
        ) && let Some(translated) = self.translate_waveform_image_if_possible(
            &request.decoded,
            previous_meta,
            previous_image,
            &request.meta,
        ) {
            self.store_waveform_image(translated, request.meta);
            return;
        }
        let color_image = self
            .sample_view
            .renderer
            .render_color_image_for_view_with_size_and_fade_and_transients(
                &request.decoded,
                self.ui.waveform.channel_view,
                crate::waveform::WaveformRenderViewport {
                    size: [request.effective_width, request.meta.size[1]],
                    view_start: request.view.start as f32,
                    view_end: request.view.end as f32,
                    edit_fade: request.meta.edit_fade,
                },
                request
                    .meta
                    .transient_visual_token
                    .map(|_| self.ui.waveform.transients.as_ref()),
            );
        // Keep waveform image metadata in the renderer to preserve precision.
        self.store_waveform_image(color_image, request.meta);
    }

    /// Queue the latest waveform image render without blocking the GUI thread.
    fn queue_waveform_render_now(&mut self) {
        let Some(request) = self.plan_waveform_render_request() else {
            return;
        };

        if self
            .sample_view
            .waveform
            .render_meta
            .as_ref()
            .is_some_and(|meta: &WaveformRenderMeta| meta.matches(&request.meta))
        {
            return;
        }
        if let (Some(previous_meta), Some(previous_image)) = (
            self.sample_view.waveform.render_meta.as_ref(),
            self.ui.waveform.image.as_ref(),
        ) && let Some(translated) = self.translate_waveform_image_if_possible(
            &request.decoded,
            previous_meta,
            previous_image,
            &request.meta,
        ) {
            self.store_waveform_image(translated, request.meta);
            return;
        }
        let request_id = self.runtime.jobs.next_waveform_render_request_id();
        let key = request.key();
        if self
            .runtime
            .waveform
            .pending_render
            .as_ref()
            .is_some_and(|pending| pending.key == key)
        {
            return;
        }
        self.runtime.waveform.pending_render = Some(PendingWaveformRender {
            request_id,
            key,
            queued_at: Instant::now(),
        });
        self.runtime
            .jobs
            .publish_latest_waveform_render_request_id(request_id);
        let job = WaveformRenderJob {
            request_id,
            key,
            decoded: request.decoded,
            renderer: self.sample_view.renderer.clone(),
            channel_view: self.ui.waveform.channel_view,
            viewport: crate::waveform::WaveformRenderViewport {
                size: [request.effective_width, request.meta.size[1]],
                view_start: request.view.start as f32,
                view_end: request.view.end as f32,
                edit_fade: request.meta.edit_fade,
            },
            transients: request
                .meta
                .transient_visual_token
                .map(|_| self.ui.waveform.transients.clone()),
        };
        let latest_request_id = self.runtime.jobs.latest_waveform_render_request_tracker();
        self.runtime
            .jobs
            .spawn_optional_one_shot_job(true, move || {
                worker_jobs::run_waveform_render_job(job, request.meta, latest_request_id)
                    .map(JobMessage::WaveformRendered)
            });
        self.mark_waveform_projection_dirty();
        self.mark_derived_source_dirty(DerivedNodeId::StatusState, DirtyReason::StatusAction);
    }

    fn plan_waveform_render_request(&mut self) -> Option<WaveformRenderRequest> {
        let decoded = self.sample_view.waveform.decoded.as_ref().cloned()?;
        let [width, height] = self.sample_view.waveform.size;
        let total_frames = decoded.frame_count();
        let view = self.ui.waveform.view.clamp();
        let target = width
            .saturating_mul(WAVEFORM_RENDER_SUPERSAMPLE_X)
            .min(super::MAX_TEXTURE_WIDTH) as usize;

        if (decoded.samples.is_empty() && decoded.peaks.is_none()) || total_frames == 0 {
            self.clear_waveform_render_state();
            return None;
        }
        let start_frame = ((view.start * total_frames as f64).floor() as usize)
            .min(total_frames.saturating_sub(1));
        let mut end_frame =
            ((view.end * total_frames as f64).ceil() as usize).clamp(start_frame + 1, total_frames);
        if end_frame <= start_frame {
            end_frame = (start_frame + 1).min(total_frames);
        }
        let frames_in_view = end_frame.saturating_sub(start_frame).max(1);
        let upper_width = frames_in_view.min(super::MAX_TEXTURE_WIDTH as usize);
        let lower_bound = width.min(super::MAX_TEXTURE_WIDTH) as usize;
        let max_texture_width = upper_width.max(lower_bound) as u32;
        let raw_texture_width = target.min(upper_width).max(lower_bound) as u32;
        let effective_width = reuse::stabilized_texture_width(
            raw_texture_width,
            lower_bound as u32,
            max_texture_width,
            self.sample_view
                .waveform
                .render_meta
                .as_ref()
                .map(|meta| meta.texture_width),
        );
        let meta = WaveformRenderMeta {
            view_start: view.start,
            view_end: view.end,
            size: [width, height],
            samples_len: total_frames,
            texture_width: effective_width,
            channel_view: self.ui.waveform.channel_view,
            channels: decoded.channels,
            edit_fade: self
                .ui
                .waveform
                .edit_selection
                .filter(|selection| selection.has_edit_effects()),
            transient_visual_token: self
                .ui
                .waveform
                .transient_cache_token
                .filter(|_| self.ui.waveform.transient_markers_enabled),
        };
        Some(WaveformRenderRequest {
            decoded,
            meta,
            view,
            effective_width,
        })
    }

    pub(crate) fn waveform_render_key_matches_current_view(
        &mut self,
        key: WaveformRenderKey,
    ) -> bool {
        self.plan_waveform_render_request()
            .is_some_and(|request| request.key() == key)
    }

    fn clear_waveform_render_state(&mut self) {
        self.ui.waveform.image = None;
        self.ui.waveform.waveform_image_signature = None;
        self.projected_waveform_image_signature = None;
        self.projected_waveform_image = None;
        self.runtime.waveform.pending_render = None;
        self.mark_waveform_projection_dirty();
        self.mark_derived_source_dirty(DerivedNodeId::StatusState, DirtyReason::StatusAction);
    }
}

impl WaveformRenderRequest {
    fn key(&self) -> WaveformRenderKey {
        WaveformRenderKey {
            cache_token: self.decoded.cache_token,
            texture_width: self.effective_width,
            height: self.meta.size[1],
            channel_view: self.meta.channel_view,
            view_start_bits: self.view.start.to_bits(),
            view_end_bits: self.view.end.to_bits(),
            transient_visual_token: self.meta.transient_visual_token,
        }
    }
}

fn waveform_render_async_enabled() -> bool {
    #[cfg(test)]
    {
        false
    }
    #[cfg(not(test))]
    {
        true
    }
}
