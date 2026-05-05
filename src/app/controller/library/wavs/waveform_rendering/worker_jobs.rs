use super::*;
use crate::app::controller::jobs::{
    WaveformRenderJob, WaveformRenderResult, WaveformTransientResult,
};
use crate::waveform::DecodedWaveform;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

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

pub(crate) fn run_waveform_transient_job(
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
