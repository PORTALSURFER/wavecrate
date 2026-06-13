use super::*;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::controller::AppControllerUiRuntimeExt;

#[test]
fn batched_zoom_matches_sequential_steps() {
    let (mut batched, source_a) = dummy_controller();
    prepare_browser_sample(&mut batched, &source_a, "zoom.wav");
    batched.update_waveform_size(240, 24);
    batched.select_wav_by_path(Path::new("zoom.wav"));
    batched.ui.waveform.playhead.position = 0.4;
    batched.ui.waveform.playhead.visible = true;

    let (mut stepped, source_b) = dummy_controller();
    prepare_browser_sample(&mut stepped, &source_b, "zoom.wav");
    stepped.update_waveform_size(240, 24);
    stepped.select_wav_by_path(Path::new("zoom.wav"));
    stepped.ui.waveform.playhead.position = 0.4;
    stepped.ui.waveform.playhead.visible = true;

    batched.zoom_waveform_steps(true, 3, None);
    for _ in 0..3 {
        stepped
            .waveform()
            .apply_zoom_step(true, None, None, false, false);
    }

    let view_a = batched.ui.waveform.view;
    let view_b = stepped.ui.waveform.view;
    assert!((view_a.start - view_b.start).abs() < 1e-6);
    assert!((view_a.end - view_b.end).abs() < 1e-6);
}

#[test]
fn batched_zoom_many_steps_matches_sequential_steps() {
    let (mut batched, source_a) = dummy_controller();
    prepare_browser_sample(&mut batched, &source_a, "zoom-many.wav");
    batched.update_waveform_size(240, 24);
    batched.select_wav_by_path(Path::new("zoom-many.wav"));
    batched.ui.controls.keyboard_zoom_factor = 0.5;

    let (mut stepped, source_b) = dummy_controller();
    prepare_browser_sample(&mut stepped, &source_b, "zoom-many.wav");
    stepped.update_waveform_size(240, 24);
    stepped.select_wav_by_path(Path::new("zoom-many.wav"));
    stepped.ui.controls.keyboard_zoom_factor = 0.5;

    batched.zoom_waveform_steps(true, 12, None);
    for _ in 0..12 {
        stepped
            .waveform()
            .apply_zoom_step(true, None, None, false, false);
    }

    let view_a = batched.ui.waveform.view;
    let view_b = stepped.ui.waveform.view;
    assert!((view_a.start - view_b.start).abs() < 1e-6);
    assert!((view_a.end - view_b.end).abs() < 1e-6);
}

#[test]
fn mouse_zoom_prefers_pointer_over_playhead() {
    let (mut controller, _source) = dummy_controller();
    controller.sample_view.waveform.size = [240, 24];
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.playhead.position = 0.1;
    controller.ui.waveform.playhead.visible = true;

    controller.zoom_waveform_steps_with_factor(true, 1, Some(0.8), Some(0.5), false, false);

    let center = (controller.ui.waveform.view.start + controller.ui.waveform.view.end) * 0.5;
    let playhead_dist = (center - 0.1).abs();
    let pointer_dist = (center - 0.8).abs();
    assert!(
        pointer_dist < playhead_dist,
        "zoom centered closer to playhead ({playhead_dist}) than pointer ({pointer_dist}), center {center}"
    );
    assert!(controller.ui.waveform.view.start < controller.ui.waveform.view.end);
}

#[test]
fn selection_action_resolves_to_stable_frame_bounds_across_zoom_levels() {
    let cases = [
        (0.0, 1.0, 0.125, 0.875),
        (0.2, 0.6, 0.333_333, 0.666_667),
        (0.500_1, 0.500_4, 0.25, 0.75),
    ];

    for (view_start, view_end, start_ratio, end_ratio) in cases {
        let (mut controller, _source) = dummy_controller();
        install_decoded_waveform(&mut controller);
        controller.ui.waveform.view = crate::app::state::WaveformView {
            start: view_start,
            end: view_end,
        };
        let start_micros = view_pointer_micros(view_start, view_end, start_ratio);
        let end_micros = view_pointer_micros(view_start, view_end, end_ratio);

        controller.apply_ui_action(NativeUiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override: true,
                preserve_view_edge: false,
            },
        ));
        let selection = controller.ui.waveform.selection.expect("selection");
        let first_bounds = selection.frame_bounds(10_000);

        controller.zoom_waveform_steps(true, 8, Some(selection.start_f64()));

        assert_eq!(controller.ui.waveform.selection, Some(selection));
        assert_eq!(selection.frame_bounds(10_000), first_bounds);
        assert_eq!(
            first_bounds,
            expected_frame_bounds(start_micros, end_micros, 10_000)
        );
    }
}

#[test]
fn selection_frame_bounds_survive_pointer_zoom_pan_and_projection_refreshes() {
    let cases = [
        ("full", 0.0, 1.0, 0.18, 0.42),
        ("zoomed", 0.2, 0.7, 0.25, 0.55),
        ("deep", 0.500_100, 0.500_700, 0.2, 0.8),
    ];

    for (label, view_start, view_end, start_ratio, end_ratio) in cases {
        let (mut controller, _source) = dummy_controller();
        install_decoded_waveform(&mut controller);
        controller.ui.waveform.view = crate::app::state::WaveformView {
            start: view_start,
            end: view_end,
        };
        let start_micros = view_pointer_micros(view_start, view_end, start_ratio);
        let end_micros = view_pointer_micros(view_start, view_end, end_ratio);

        controller.apply_ui_action(NativeUiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override: true,
                preserve_view_edge: false,
            },
        ));

        let expected = expected_frame_bounds(start_micros, end_micros, 10_000);
        assert_selection_frame_bounds(&controller, expected, label);
        let selection = controller.ui.waveform.selection.expect("selection");

        for anchor_ratio_micros in [100_000, 500_000, 900_000] {
            controller.apply_ui_action(NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                    zoom_in: true,
                    steps: 2,
                    anchor_ratio_micros: Some(anchor_ratio_micros),
                },
            ));
            assert_selection_frame_bounds(&controller, expected, label);

            controller.apply_ui_action(NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                    zoom_in: false,
                    steps: 1,
                    anchor_ratio_micros: Some(anchor_ratio_micros),
                },
            ));
            assert_selection_frame_bounds(&controller, expected, label);
        }

        for center_nanos in [250_000_000, 500_000_050, 750_000_000] {
            controller.apply_ui_action(NativeUiAction::Waveform(
                crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter {
                    center_micros: (center_nanos / 1_000).min(1_000_000),
                    center_nanos: Some(center_nanos),
                },
            ));
            assert_selection_frame_bounds(&controller, expected, label);
        }

        controller.refresh_waveform_image();
        assert_eq!(
            controller.ui.waveform.selection,
            Some(selection),
            "{label}: projection refresh should not rewrite selection endpoints"
        );
        assert_selection_frame_bounds(&controller, expected, label);
    }
}

#[test]
fn repeated_pointer_zoom_at_minimum_view_width_is_a_noop_for_selection_and_view() {
    let (mut controller, _source) = dummy_controller();
    install_decoded_waveform(&mut controller);
    controller.update_waveform_size(240, 24);
    controller.ui.controls.keyboard_zoom_factor = 0.5;
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.45,
        end: 0.57,
    };

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 480_000,
            end_micros: 520_000,
            snap_override: true,
            preserve_view_edge: false,
        },
    ));
    let expected_selection = controller.ui.waveform.selection.expect("selection");
    let expected_bounds = expected_selection.frame_bounds(10_000);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: Some(250_000),
        },
    ));

    let clamped_view = controller.ui.waveform.view;
    assert!(
        (clamped_view.width() - 0.096).abs() < 1.0e-9,
        "240px waveform with 4x render target over 10k frames should clamp to 960 frames, got {}",
        clamped_view.width()
    );
    let anchor = 0.48;
    let after_ratio = (anchor - clamped_view.start) / clamped_view.width();
    assert!((after_ratio - 0.25).abs() < 1.0e-9);

    for _ in 0..12 {
        controller.apply_ui_action(NativeUiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::ZoomWaveform {
                zoom_in: true,
                steps: 1,
                anchor_ratio_micros: Some(250_000),
            },
        ));
    }

    assert_eq!(controller.ui.waveform.view, clamped_view);
    assert_eq!(controller.ui.waveform.selection, Some(expected_selection));
    assert_eq!(
        controller
            .ui
            .waveform
            .selection
            .expect("selection")
            .frame_bounds(10_000),
        expected_bounds
    );
}

#[test]
fn edit_selection_action_resolves_to_stable_frame_bounds_at_deep_zoom() {
    let (mut controller, _source) = dummy_controller();
    install_decoded_waveform(&mut controller);
    controller.ui.waveform.view = crate::app::state::WaveformView {
        start: 0.812_345,
        end: 0.812_745,
    };
    let start_micros = view_pointer_micros(0.812_345, 0.812_745, 0.2);
    let end_micros = view_pointer_micros(0.812_345, 0.812_745, 0.8);

    controller.apply_ui_action(NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge: false,
        },
    ));

    let selection = controller
        .ui
        .waveform
        .edit_selection
        .expect("edit selection");
    assert_eq!(
        selection.frame_bounds(10_000),
        expected_frame_bounds(start_micros, end_micros, 10_000)
    );
}

fn assert_selection_frame_bounds(
    controller: &AppController,
    expected: crate::selection::SampleFrameRange,
    label: &str,
) {
    let selection = controller.ui.waveform.selection.expect("selection");
    assert_eq!(
        selection.frame_bounds(10_000),
        expected,
        "{label}: selection frame bounds changed after viewport interaction"
    );
}

fn view_pointer_micros(view_start: f64, view_end: f64, pointer_ratio: f64) -> u32 {
    ((view_start + ((view_end - view_start) * pointer_ratio)).clamp(0.0, 1.0) * 1_000_000.0).round()
        as u32
}

fn expected_frame_bounds(
    start_micros: u32,
    end_micros: u32,
    total_frames: usize,
) -> crate::selection::SampleFrameRange {
    crate::selection::SelectionRange::from_precise_normalized_frame_bounds(
        total_frames,
        f64::from(start_micros) / 1_000_000.0,
        f64::from(end_micros) / 1_000_000.0,
    )
    .frame_bounds(total_frames)
}
