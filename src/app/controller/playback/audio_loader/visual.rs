use super::{
    AudioTransientResult, AudioVisualResult, PendingTransientCompute,
    pending::PendingVisualCompute,
    stages,
    telemetry::{StaleDropStage, stale_and_record},
};
use crate::app::controller::library::wavs::waveform_rendering::{
    PreparedWaveformVisual, prepare_initial_waveform_visual,
};
use crate::waveform::WaveformRenderer;
use std::sync::{Arc, atomic::AtomicU64};

pub(super) fn build_visual_result(
    renderer: &WaveformRenderer,
    pending: PendingVisualCompute,
    latest_request_id: &AtomicU64,
) -> Option<AudioVisualResult> {
    let transients = match pending.known_transients {
        Some(transients) => transients,
        None => {
            let result: AudioTransientResult = stages::build_transient_result(
                PendingTransientCompute {
                    request_id: pending.request_id,
                    source_id: pending.source_id.clone(),
                    relative_path: pending.relative_path.clone(),
                    metadata: pending.metadata,
                    cache_token: pending.cache_token,
                    decoded: Arc::clone(&pending.decoded),
                    stretched: pending.stretched,
                },
                latest_request_id,
            )?;
            result.transients
        }
    };
    if stale_and_record(
        pending.request_id,
        latest_request_id,
        StaleDropStage::PostTransients,
    ) {
        return None;
    }
    let PreparedWaveformVisual {
        image,
        projected_image,
        render_meta,
    } = prepare_initial_waveform_visual(
        renderer,
        pending.decoded.as_ref(),
        pending.render_spec,
        transients.as_ref(),
    );
    if stale_and_record(
        pending.request_id,
        latest_request_id,
        StaleDropStage::PreSend,
    ) {
        return None;
    }
    Some(AudioVisualResult {
        request_id: pending.request_id,
        source_id: pending.source_id,
        relative_path: pending.relative_path,
        metadata: pending.metadata,
        cache_token: pending.cache_token,
        transients,
        image,
        projected_image,
        render_meta,
        stretched: pending.stretched,
    })
}
