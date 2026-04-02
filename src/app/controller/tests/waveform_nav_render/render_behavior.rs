use super::*;

#[test]
fn waveform_refresh_respects_view_slice_and_caps_width() {
    let (mut controller, _source) = dummy_controller();
    controller.sample_view.waveform.size = [100, 10];
    controller.ui.waveform.view = WaveformView {
        start: 0.25,
        end: 0.5,
    };
    controller.sample_view.waveform.decoded = Some(std::sync::Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: std::sync::Arc::from((0..1000).map(|i| i as f32).collect::<Vec<_>>()),
        analysis_samples: std::sync::Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
    controller.sample_view.waveform.render_meta = None;
    controller.refresh_waveform_image();
    let image = controller
        .ui
        .waveform
        .image
        .as_ref()
        .expect("waveform image");
    let render_meta = controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("render metadata");
    assert!((render_meta.view_start - 0.25).abs() < 1e-6);
    assert!((render_meta.view_end - 0.5).abs() < 1e-6);
    let expected_width = controller.sample_view.waveform.size[0]
        .saturating_mul(TEST_WAVEFORM_RENDER_SUPERSAMPLE_X)
        .min(crate::app::controller::library::wavs::MAX_TEXTURE_WIDTH)
        as usize;
    let samples_in_view = (0.5 - 0.25) * 1000.0;
    let upper = (samples_in_view as usize)
        .min(crate::app::controller::library::wavs::MAX_TEXTURE_WIDTH as usize)
        .max(1);
    let lower = controller.sample_view.waveform.size[0]
        .min(crate::app::controller::library::wavs::MAX_TEXTURE_WIDTH) as usize;
    let clamped = expected_width.min(upper).max(lower);
    assert_eq!(image.size[0], clamped);
    assert_eq!(image.size[1], 10);
}

#[test]
fn waveform_render_meta_rejects_small_shifts_when_zoomed_in() {
    let base = wavs::WaveformRenderMeta {
        view_start: 0.10,
        view_end: 0.1009,
        size: [240, 32],
        samples_len: 10_000,
        texture_width: 8_000,
        channel_view: crate::waveform::WaveformChannelView::Mono,
        channels: 2,
        edit_fade: None,
        transient_visual_token: None,
    };
    let shifted = wavs::WaveformRenderMeta {
        view_start: 0.10095,
        view_end: 0.10185,
        ..base
    };
    assert!(!base.matches(&shifted));
}

#[test]
fn waveform_render_meta_allows_small_shifts_on_full_view() {
    let base = wavs::WaveformRenderMeta {
        view_start: 0.0,
        view_end: 1.0,
        size: [240, 32],
        samples_len: 10_000,
        texture_width: 2_000,
        channel_view: crate::waveform::WaveformChannelView::Mono,
        channels: 1,
        edit_fade: None,
        transient_visual_token: None,
    };
    let minor_shift = wavs::WaveformRenderMeta {
        view_start: 0.0005,
        view_end: 1.0005,
        ..base
    };
    assert!(base.matches(&minor_shift));
}

#[test]
fn waveform_render_meta_rejects_transient_visual_token_changes() {
    let base = wavs::WaveformRenderMeta {
        view_start: 0.0,
        view_end: 1.0,
        size: [240, 32],
        samples_len: 10_000,
        texture_width: 2_000,
        channel_view: crate::waveform::WaveformChannelView::Mono,
        channels: 1,
        edit_fade: None,
        transient_visual_token: Some(7),
    };
    let changed = wavs::WaveformRenderMeta {
        transient_visual_token: Some(8),
        ..base
    };
    let disabled = wavs::WaveformRenderMeta {
        transient_visual_token: None,
        ..base
    };
    assert!(!base.matches(&changed));
    assert!(!base.matches(&disabled));
}

#[test]
/// Adjacent panning should keep translated renders visually close to full rerenders.
fn adjacent_pan_translation_matches_full_render_output() {
    let (mut controller, _source) = dummy_controller();
    controller.sample_view.waveform.size = [200, 24];
    controller.ui.waveform.view = WaveformView {
        start: 0.20,
        end: 0.60,
    };
    controller.sample_view.waveform.decoded = Some(std::sync::Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: std::sync::Arc::from(
            (0..2_000)
                .map(|index| ((index as f32 * 0.017).sin() * 0.9).clamp(-1.0, 1.0))
                .collect::<Vec<_>>(),
        ),
        analysis_samples: std::sync::Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));

    controller.refresh_waveform_image();
    controller.ui.waveform.view = WaveformView {
        start: 0.202,
        end: 0.602,
    };
    controller.refresh_waveform_image();

    let render_meta = *controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("render metadata");
    let actual = controller
        .ui
        .waveform
        .image
        .as_ref()
        .expect("translated waveform image")
        .clone();
    let decoded = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform");
    let expected = controller
        .sample_view
        .renderer
        .render_color_image_for_view_with_size_and_fade(
            decoded,
            controller.ui.waveform.channel_view,
            crate::waveform::WaveformRenderViewport {
                size: [render_meta.texture_width, render_meta.size[1]],
                view_start: controller.ui.waveform.view.start as f32,
                view_end: controller.ui.waveform.view.end as f32,
                edit_fade: render_meta.edit_fade,
            },
        );
    assert_eq!(actual.size, expected.size);
    let mismatched = actual
        .pixels
        .iter()
        .zip(expected.pixels.iter())
        .filter(|(left, right)| left != right)
        .count();
    let mismatch_ratio = mismatched as f64 / actual.pixels.len().max(1) as f64;
    assert!(
        mismatch_ratio <= 0.25,
        "expected translated pan render to stay close to full render; mismatch ratio={mismatch_ratio:.3}"
    );
}

#[test]
/// Adjacent viewport sizes should retain stable texture-width bucketing.
fn waveform_texture_width_is_stable_for_adjacent_sizes() {
    let (mut controller, _source) = dummy_controller();
    controller.sample_view.waveform.decoded = Some(std::sync::Arc::new(DecodedWaveform {
        cache_token: 1,
        samples: std::sync::Arc::from(
            (0..4_096)
                .map(|index| ((index as f32 * 0.023).sin() * 0.8).clamp(-1.0, 1.0))
                .collect::<Vec<_>>(),
        ),
        analysis_samples: std::sync::Arc::from(Vec::new()),
        analysis_sample_rate: 0,
        analysis_stride: 1,
        peaks: None,
        duration_seconds: 1.0,
        sample_rate: 48_000,
        channels: 1,
    }));
    controller.ui.waveform.view = WaveformView::default();

    controller.update_waveform_size(511, 24);
    let first_width = controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("first render metadata")
        .texture_width;

    controller.update_waveform_size(512, 24);
    let second_width = controller
        .sample_view
        .waveform
        .render_meta
        .as_ref()
        .expect("second render metadata")
        .texture_width;

    assert_eq!(first_width, second_width);
}
