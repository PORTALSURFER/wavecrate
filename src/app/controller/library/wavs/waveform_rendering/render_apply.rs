use super::*;
use crate::app::controller::jobs::{
    JobMessage, WaveformRenderJob, WaveformRenderKey, WaveformRenderResult,
    WaveformTransientResult,
};
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::app::controller::state::runtime::PendingWaveformTransientCompute;
use crate::app::state::WaveformView;
use crate::waveform::DecodedWaveform;
use std::fs;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

fn min_view_width_for_frames(frame_count: usize, width_px: u32) -> f64 {
    if frame_count == 0 {
        return 1.0;
    }
    let samples = frame_count as f64;
    let pixels = width_px.max(1) as f64;
    (pixels * MIN_SAMPLES_PER_PIXEL as f64 / samples).clamp(MIN_VIEW_WIDTH_BASE, 1.0)
}

impl AppController {
    pub(crate) fn min_view_width(&self) -> f64 {
        if let Some(decoded) = self.sample_view.waveform.decoded.as_ref() {
            min_view_width_for_frames(decoded.frame_count(), self.sample_view.waveform.size[0])
        } else {
            MIN_VIEW_WIDTH_BASE
        }
    }

    /// Apply waveform payloads using shared immutable buffers.
    pub(crate) fn apply_waveform_image_shared(
        &mut self,
        decoded: Arc<DecodedWaveform>,
        transients: Option<Arc<[f32]>>,
    ) {
        if self
            .sample_view
            .waveform
            .decoded
            .as_ref()
            .is_some_and(|d| d.cache_token == decoded.cache_token)
        {
            // Content matches, no need to invalidate the current render or transients.
            self.sample_view.waveform.decoded = Some(decoded);
            return;
        }

        let token = decoded.cache_token;
        // Force a rerender whenever decoded samples change, even if the view metadata is
        // identical to the previous render.
        self.sample_view.waveform.render_meta = None;
        self.sample_view.waveform.decoded = Some(decoded);
        self.runtime.pending_waveform_transient_compute = None;

        // Reset view to show full waveform when loading new audio
        self.ui.waveform.view = WaveformView {
            start: 0.0,
            end: 1.0,
        };

        if let Some(transients) = transients {
            self.runtime.jobs.invalidate_waveform_transient_requests();
            self.ui.waveform.transients = transients;
            self.ui.waveform.transient_cache_token = Some(token);
        } else {
            self.ui.waveform.transients = Arc::from([]);
            self.ui.waveform.transient_cache_token = None;
            self.queue_waveform_transient_refresh(Arc::clone(
                self.sample_view
                    .waveform
                    .decoded
                    .as_ref()
                    .expect("decoded waveform should be present after assignment"),
            ));
        }
        self.refresh_waveform_image_with_reason(WaveformRefreshReason::Data);
    }

    /// Apply waveform payloads using owned values.
    ///
    /// This compatibility path adapts legacy call sites to the shared immutable
    /// payload pipeline and should be removed once all callers are Arc-first.
    pub(crate) fn apply_waveform_image(
        &mut self,
        decoded: DecodedWaveform,
        transients: Option<Vec<f32>>,
    ) {
        self.apply_waveform_image_shared(Arc::new(decoded), transients.map(Arc::from));
    }

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
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref() else {
            return;
        };
        let [width, height] = self.sample_view.waveform.size;
        let total_frames = decoded.frame_count();
        let view = self.ui.waveform.view.clamp();
        let target = width
            .saturating_mul(WAVEFORM_RENDER_SUPERSAMPLE_X)
            .min(super::MAX_TEXTURE_WIDTH) as usize;

        if (decoded.samples.is_empty() && decoded.peaks.is_none()) || total_frames == 0 {
            self.ui.waveform.image = None;
            self.ui.waveform.waveform_image_signature = None;
            self.projected_waveform_image_signature = None;
            self.projected_waveform_image = None;
            self.mark_waveform_projection_dirty();
            return;
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
        let desired_meta = WaveformRenderMeta {
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
        if self
            .sample_view
            .waveform
            .render_meta
            .as_ref()
            .is_some_and(|meta: &WaveformRenderMeta| meta.matches(&desired_meta))
        {
            return;
        }
        if let (Some(previous_meta), Some(previous_image)) = (
            self.sample_view.waveform.render_meta.as_ref(),
            self.ui.waveform.image.as_ref(),
        ) && let Some(translated) = self.translate_waveform_image_if_possible(
            decoded,
            previous_meta,
            previous_image,
            &desired_meta,
        ) {
            self.store_waveform_image(translated, desired_meta);
            return;
        }
        let color_image = self
            .sample_view
            .renderer
            .render_color_image_for_view_with_size_and_fade_and_transients(
                decoded,
                self.ui.waveform.channel_view,
                crate::waveform::WaveformRenderViewport {
                    size: [effective_width, height],
                    view_start: view.start as f32,
                    view_end: view.end as f32,
                    edit_fade: desired_meta.edit_fade,
                },
                desired_meta
                    .transient_visual_token
                    .map(|_| self.ui.waveform.transients.as_ref()),
            );
        // Keep waveform image metadata in the renderer to preserve precision.
        self.store_waveform_image(color_image, desired_meta);
    }

    /// Queue the latest waveform image render without blocking the GUI thread.
    fn queue_waveform_render_now(&mut self) {
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref().cloned() else {
            return;
        };
        let [width, height] = self.sample_view.waveform.size;
        let total_frames = decoded.frame_count();
        let view = self.ui.waveform.view.clamp();
        let target = width
            .saturating_mul(WAVEFORM_RENDER_SUPERSAMPLE_X)
            .min(super::MAX_TEXTURE_WIDTH) as usize;

        if (decoded.samples.is_empty() && decoded.peaks.is_none()) || total_frames == 0 {
            self.ui.waveform.image = None;
            self.ui.waveform.waveform_image_signature = None;
            self.projected_waveform_image_signature = None;
            self.projected_waveform_image = None;
            self.runtime.pending_waveform_render = None;
            self.mark_waveform_projection_dirty();
            return;
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
        let desired_meta = WaveformRenderMeta {
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
        if self
            .sample_view
            .waveform
            .render_meta
            .as_ref()
            .is_some_and(|meta: &WaveformRenderMeta| meta.matches(&desired_meta))
        {
            return;
        }
        let request_id = self.runtime.jobs.next_waveform_render_request_id();
        let key = WaveformRenderKey {
            cache_token: decoded.cache_token,
            texture_width: effective_width,
            height,
            channel_view: self.ui.waveform.channel_view,
            view_start_bits: view.start.to_bits(),
            view_end_bits: view.end.to_bits(),
            transient_visual_token: desired_meta.transient_visual_token,
        };
        if self
            .runtime
            .pending_waveform_render
            .as_ref()
            .is_some_and(|pending| pending.key == key)
        {
            return;
        }
        self.runtime.pending_waveform_render = Some(
            crate::app::controller::state::runtime::PendingWaveformRender {
                request_id,
                key,
                queued_at: Instant::now(),
            },
        );
        self.runtime
            .jobs
            .publish_latest_waveform_render_request_id(request_id);
        let job = WaveformRenderJob {
            request_id,
            key,
            decoded,
            renderer: self.sample_view.renderer.clone(),
            channel_view: self.ui.waveform.channel_view,
            viewport: crate::waveform::WaveformRenderViewport {
                size: [effective_width, height],
                view_start: view.start as f32,
                view_end: view.end as f32,
                edit_fade: desired_meta.edit_fade,
            },
            transients: desired_meta
                .transient_visual_token
                .map(|_| self.ui.waveform.transients.clone()),
        };
        let latest_request_id = self.runtime.jobs.latest_waveform_render_request_tracker();
        self.runtime.jobs.spawn_optional_one_shot_job(
            true,
            move || {
                run_waveform_render_job(job, desired_meta, latest_request_id)
                    .map(JobMessage::WaveformRendered)
            },
        );
        self.mark_waveform_projection_dirty();
    }

    fn queue_waveform_transient_refresh(&mut self, decoded: Arc<DecodedWaveform>) {
        if self.ui.waveform.transient_cache_token == Some(decoded.cache_token) {
            return;
        }
        if self
            .runtime
            .pending_waveform_transient_compute
            .as_ref()
            .is_some_and(|pending| pending.cache_token == decoded.cache_token)
        {
            return;
        }
        let request_id = self.runtime.jobs.next_waveform_transient_request_id();
        self.runtime.pending_waveform_transient_compute = Some(PendingWaveformTransientCompute {
            request_id,
            cache_token: decoded.cache_token,
            queued_at: Instant::now(),
        });
        self.runtime
            .jobs
            .publish_latest_waveform_transient_request_id(request_id);
        let latest_request_id = self.runtime.jobs.latest_waveform_transient_request_tracker();
        self.runtime.jobs.spawn_optional_one_shot_job(true, move || {
            run_waveform_transient_job(request_id, decoded, latest_request_id)
                .map(JobMessage::WaveformTransientsComputed)
        });
    }

    pub(crate) fn refresh_waveform_transients(&mut self) {
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref().cloned() else {
            self.ui.waveform.transients = Arc::from([]);
            self.ui.waveform.transient_cache_token = None;
            return;
        };
        self.ui.waveform.transients = Arc::from([]);
        self.ui.waveform.transient_cache_token = None;
        self.queue_waveform_transient_refresh(decoded);
    }

    pub(crate) fn read_waveform_bytes(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<Vec<u8>, String> {
        let full_path = source.root.join(relative_path);
        let bytes = fs::read(&full_path)
            .map_err(|err| format!("Failed to read {}: {err}", full_path.display()))?;
        Ok(crate::wav_sanitize::sanitize_wav_bytes(bytes))
    }

    pub(crate) fn current_file_metadata(
        &self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<FileMetadata, String> {
        let full_path = source.root.join(relative_path);
        let metadata = fs::metadata(&full_path)
            .map_err(|err| format!("Failed to read {}: {err}", full_path.display()))?;
        let modified_ns = metadata
            .modified()
            .map_err(|err| format!("Missing modified time for {}: {err}", full_path.display()))?
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map_err(|_| "File modified time is before epoch".to_string())?
            .as_nanos() as i64;
        Ok(FileMetadata {
            file_size: metadata.len(),
            modified_ns,
        })
    }
}

/// Execute one waveform render request on a background worker thread.
pub(crate) fn run_waveform_render_job(
    job: WaveformRenderJob,
    render_meta: WaveformRenderMeta,
    latest_request_id: Arc<AtomicU64>,
) -> Option<WaveformRenderResult> {
    if latest_request_is_stale(&job.request_id, &latest_request_id) {
        return None;
    }
    let started_at = Instant::now();
    let image = job
        .renderer
        .render_color_image_for_view_with_size_and_fade_and_transients(
            &job.decoded,
            job.channel_view,
            job.viewport,
            job.transients.as_deref(),
        );
    if latest_request_is_stale(&job.request_id, &latest_request_id) {
        return None;
    }
    let projected_image = super::waveform_image_to_native_rgba(&image);
    Some(WaveformRenderResult {
        request_id: job.request_id,
        key: job.key,
        elapsed: started_at.elapsed(),
        result: Ok(super::PreparedWaveformVisual {
            image: Some(image),
            projected_image,
            render_meta: Some(render_meta),
        }),
    })
}

fn run_waveform_transient_job(
    request_id: u64,
    decoded: Arc<DecodedWaveform>,
    latest_request_id: Arc<AtomicU64>,
) -> Option<WaveformTransientResult> {
    if latest_request_is_stale(&request_id, &latest_request_id) {
        return None;
    }
    let started_at = Instant::now();
    let transients: Arc<[f32]> =
        crate::waveform::transients::detect_transients(&decoded, DEFAULT_TRANSIENT_SENSITIVITY)
            .into();
    if latest_request_is_stale(&request_id, &latest_request_id) {
        return None;
    }
    Some(WaveformTransientResult {
        request_id,
        cache_token: decoded.cache_token,
        elapsed: started_at.elapsed(),
        result: Ok(transients),
    })
}

fn latest_request_is_stale(request_id: &u64, latest_request_id: &Arc<AtomicU64>) -> bool {
    latest_request_id.load(Ordering::Relaxed) != *request_id
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
