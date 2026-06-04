use super::*;

#[test]
fn signal_widget_paints_gpu_surface_without_app_overlay_handles() {
    let state = WaveformState::synthetic_for_tests();
    let plan = waveform_signal_surface_plan(state.file(), state.viewport(), state.edit_selection());

    let surface = plan
        .gpu_surfaces()
        .find(|surface| {
            matches!(
                surface.content,
                GpuSurfaceContent::SignalSummaryBands { .. }
            )
        })
        .expect("waveform gpu surface");

    assert!(surface.overlays.is_empty());
}

#[test]
fn signal_widget_attaches_active_edit_fade_gain_preview() {
    let file = Arc::new(waveform_file_from_mono_samples(
        "fade-preview.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![1.0; 16],
    ));
    let viewport = super::WaveformViewport::full(file.frames);
    let edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.0, 1.0).with_fade_in(1.0, 0.0));
    let plan = waveform_signal_surface_plan(Arc::clone(&file), viewport, edit_selection);

    let surface = plan.gpu_surfaces().next().expect("waveform gpu surface");

    assert!(surface.revision > 0);
    let GpuSurfaceContent::SignalSummaryBands {
        summary,
        gain_preview,
        ..
    } = &surface.content
    else {
        panic!("expected signal summary bands");
    };
    assert!(Arc::ptr_eq(summary, &file.gpu_signal_summary));
    let preview = gain_preview.expect("edit fade gain preview");
    assert_eq!(preview.start, 0.0);
    assert_eq!(preview.end, 1.0);
    assert_eq!(preview.fade_in_length, 1.0);
    assert_eq!(preview.fade_in_curve, 0.0);
}

#[test]
fn signal_widget_revision_changes_when_same_path_audio_bytes_change() {
    let first = Arc::new(waveform_file_from_mono_samples(
        "same-path.wav".into(),
        Arc::from([1_u8, 2, 3, 4]),
        48_000,
        1,
        vec![0.25; 16],
    ));
    let second = Arc::new(waveform_file_from_mono_samples(
        "same-path.wav".into(),
        Arc::from([4_u8, 3, 2, 1]),
        48_000,
        1,
        vec![1.0; 16],
    ));

    let first_revision = gpu_surface_revision_for_file(first);
    let second_revision = gpu_surface_revision_for_file(second);

    assert_ne!(first_revision, second_revision);
}

#[test]
fn signal_widget_keeps_summary_cached_during_live_edit_fade_drag() {
    let file = Arc::new(waveform_file_from_mono_samples(
        "fade-preview.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![1.0; 16],
    ));
    let viewport = super::WaveformViewport::full(file.frames);
    let edit_selection =
        Some(wavecrate::selection::SelectionRange::new(0.0, 1.0).with_fade_in(1.0, 0.0));
    let plan = waveform_signal_surface_plan(Arc::clone(&file), viewport, edit_selection);

    let surface = plan.gpu_surfaces().next().expect("waveform gpu surface");

    assert!(surface.revision > 0);
    let GpuSurfaceContent::SignalSummaryBands {
        summary,
        gain_preview,
        ..
    } = &surface.content
    else {
        panic!("expected signal summary bands");
    };
    assert!(Arc::ptr_eq(summary, &file.gpu_signal_summary));
    assert!(gain_preview.is_some());
}
