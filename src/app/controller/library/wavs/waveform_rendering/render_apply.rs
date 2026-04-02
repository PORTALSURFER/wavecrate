use super::*;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::app::state::WaveformView;
use crate::waveform::DecodedWaveform;
use std::fs;
use std::path::Path;

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

        // Reset view to show full waveform when loading new audio
        self.ui.waveform.view = WaveformView {
            start: 0.0,
            end: 1.0,
        };

        if let Some(transients) = transients {
            self.ui.waveform.transients = transients;
            self.ui.waveform.transient_cache_token = Some(token);
        } else {
            self.refresh_waveform_transients();
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

    /// Render waveform pixels for the current view immediately.
    pub(super) fn refresh_waveform_image_now(&mut self) {
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

    pub(crate) fn refresh_waveform_transients(&mut self) {
        let Some(decoded) = self.sample_view.waveform.decoded.as_ref() else {
            self.ui.waveform.transients = Arc::from([]);
            self.ui.waveform.transient_cache_token = None;
            return;
        };
        if self.ui.waveform.transient_cache_token == Some(decoded.cache_token) {
            return;
        }
        self.ui.waveform.transients =
            crate::waveform::transients::detect_transients(decoded, DEFAULT_TRANSIENT_SENSITIVITY)
                .into();
        self.ui.waveform.transient_cache_token = Some(decoded.cache_token);
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
