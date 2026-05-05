use super::super::test_support::dummy_controller;
use crate::waveform::DecodedWaveform;
use std::sync::Arc;

#[test]
fn transient_snap_restores_after_marker_toggle() {
    let (mut controller, _source) = dummy_controller();
    controller.settings.controls.transient_markers_enabled = true;
    controller.ui.waveform.transient_markers_enabled = true;
    controller.settings.controls.transient_snap_enabled = true;
    controller.ui.waveform.transient_snap_enabled = true;

    controller.set_transient_markers_enabled(false);

    assert!(!controller.ui.waveform.transient_markers_enabled);
    assert!(!controller.ui.waveform.transient_snap_enabled);
    assert!(controller.settings.controls.transient_snap_enabled);

    controller.set_transient_markers_enabled(true);

    assert!(controller.ui.waveform.transient_markers_enabled);
    assert!(controller.ui.waveform.transient_snap_enabled);
}

#[test]
fn transient_marker_toggle_rerenders_waveform_image() {
    let (mut controller, _source) = dummy_controller();
    controller.sample_view.waveform.size = [32, 8];
    controller.sample_view.waveform.decoded = Some(Arc::new(DecodedWaveform {
        cache_token: 42,
        samples: Arc::from(
            (0..256)
                .map(|index| ((index as f32 * 0.05).sin() * 0.9).clamp(-1.0, 1.0))
                .collect::<Vec<_>>(),
        ),
        analysis_samples: Arc::from(Vec::<f32>::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
    controller.ui.waveform.transients = Arc::from(vec![0.5_f32]);
    controller.ui.waveform.transient_cache_token = Some(42);
    controller.ui.waveform.transient_markers_enabled = true;

    controller.refresh_waveform_image();
    let before_signature = controller.ui.waveform.waveform_image_signature;
    let before_meta = controller
        .sample_view
        .waveform
        .render_meta
        .expect("render meta before toggle");
    assert_eq!(before_meta.transient_visual_token, Some(42));

    controller.set_transient_markers_enabled(false);

    let after_signature = controller.ui.waveform.waveform_image_signature;
    let after_meta = controller
        .sample_view
        .waveform
        .render_meta
        .expect("render meta after toggle");
    assert_ne!(before_signature, after_signature);
    assert_eq!(after_meta.transient_visual_token, None);
}
