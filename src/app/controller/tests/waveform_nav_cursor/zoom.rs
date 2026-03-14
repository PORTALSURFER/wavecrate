use super::*;

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
