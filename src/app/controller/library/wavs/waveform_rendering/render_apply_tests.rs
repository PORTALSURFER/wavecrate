use super::*;
use crate::app::controller::jobs::{JobMessage, WaveformRenderJob, WaveformRenderKey};
use crate::app::controller::state::runtime::PendingWaveformRender;
use crate::app::controller::test_support::dummy_controller;
use crate::app::state::WaveformView;
use crate::waveform::{DecodedWaveform, WaveformChannelView, WaveformRenderViewport};
use std::sync::{Arc, atomic::AtomicU64};
use std::time::{Duration, Instant};

#[test]
/// Deferred transient work should be queued instead of running synchronously on apply.
fn apply_waveform_image_without_transients_queues_deferred_compute() {
    let (mut controller, _) = dummy_controller();
    let decoded = Arc::new(decoded_waveform(7));

    controller.apply_waveform_image_shared(Arc::clone(&decoded), None);

    assert!(controller.ui.waveform.transients.is_empty());
    assert_eq!(controller.ui.waveform.transient_cache_token, None);
    let pending = controller
        .runtime
        .pending_waveform_transient_compute
        .as_ref()
        .expect("pending transient compute");
    assert_eq!(pending.request_id, 1);
    assert_eq!(pending.cache_token, decoded.cache_token);
}

#[test]
/// Stale waveform-render workers should self-drop before doing any expensive rendering.
fn stale_waveform_render_request_returns_none() {
    let renderer = WaveformRenderer::new(32, 16);
    let decoded = Arc::new(decoded_waveform(11));
    let viewport = WaveformRenderViewport {
        size: [32, 16],
        view_start: 0.0,
        view_end: 1.0,
        edit_fade: None,
    };
    let job = WaveformRenderJob {
        request_id: 7,
        key: WaveformRenderKey {
            cache_token: decoded.cache_token,
            texture_width: 32,
            height: 16,
            channel_view: WaveformChannelView::Mono,
            view_start_bits: 0.0f64.to_bits(),
            view_end_bits: 1.0f64.to_bits(),
            transient_visual_token: None,
        },
        decoded,
        renderer,
        channel_view: WaveformChannelView::Mono,
        viewport,
        transients: None,
    };
    let meta = WaveformRenderMeta {
        view_start: 0.0,
        view_end: 1.0,
        size: [32, 16],
        samples_len: 256,
        texture_width: 32,
        channel_view: WaveformChannelView::Mono,
        channels: 1,
        edit_fade: None,
        transient_visual_token: None,
    };

    let result =
        super::worker_jobs::run_waveform_render_job(job, meta, Arc::new(AtomicU64::new(9)));

    assert!(result.is_none());
}

#[test]
/// Async render apply must reject results whose exact f64 view identity is no longer current.
fn async_waveform_render_apply_discards_deep_zoom_stale_view_identity() {
    let (mut controller, _) = dummy_controller();
    let decoded = Arc::new(decoded_waveform(13));
    controller.sample_view.waveform.size = [32, 16];
    controller.sample_view.waveform.decoded = Some(Arc::clone(&decoded));
    controller.ui.waveform.view = WaveformView {
        start: 0.500_000_001,
        end: 0.500_000_201,
    };
    let stale_key = WaveformRenderKey {
        cache_token: decoded.cache_token,
        texture_width: 32,
        height: 16,
        channel_view: WaveformChannelView::Mono,
        view_start_bits: controller.ui.waveform.view.start.to_bits(),
        view_end_bits: controller.ui.waveform.view.end.to_bits(),
        transient_visual_token: None,
    };
    controller.runtime.pending_waveform_render = Some(PendingWaveformRender {
        request_id: 3,
        key: stale_key,
        queued_at: Instant::now(),
    });
    controller.ui.waveform.view = WaveformView {
        start: 0.500_000_002,
        end: 0.500_000_202,
    };
    let stale_meta = WaveformRenderMeta {
        view_start: 0.500_000_001,
        view_end: 0.500_000_201,
        size: [32, 16],
        samples_len: decoded.frame_count(),
        texture_width: 32,
        channel_view: WaveformChannelView::Mono,
        channels: 1,
        edit_fade: None,
        transient_visual_token: None,
    };

    controller.apply_background_job_message_for_tests(JobMessage::WaveformRendered(
        crate::app::controller::jobs::WaveformRenderResult {
            request_id: 3,
            key: stale_key,
            elapsed: Duration::from_millis(1),
            result: Ok(PreparedWaveformVisual {
                image: Some(crate::waveform::WaveformImage {
                    size: [32, 16],
                    pixels: vec![
                        crate::waveform::WaveformRgba::from_rgba_unmultiplied(1, 2, 3, 4);
                        32 * 16
                    ],
                }),
                projected_image: None,
                render_meta: Some(stale_meta),
            }),
        },
    ));

    let applied_meta = controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("current render after stale discard");
    assert!(applied_meta.matches_view_identity(controller.ui.waveform.view));
    assert_ne!(
        applied_meta.view_start.to_bits(),
        stale_meta.view_start.to_bits()
    );
}

fn decoded_waveform(cache_token: u64) -> DecodedWaveform {
    DecodedWaveform {
        cache_token,
        samples: Arc::from(
            (0..256)
                .map(|index| ((index as f32 * 0.03125).sin() * 0.75).clamp(-1.0, 1.0))
                .collect::<Vec<_>>(),
        ),
        analysis_samples: Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }
}
