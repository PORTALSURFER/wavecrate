use super::*;

#[test]
fn zoomed_long_wav_refines_visible_range_independently_of_total_duration() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("long-detail.wav");
    let mut samples = vec![0_i16; 4096];
    samples[1152] = i16::MAX;
    write_test_wav_i16(&path, &samples);
    let short_path = temp.path().join("short-detail.wav");
    write_test_wav_i16(&short_path, &samples[1024..1280]);
    let short = super::super::load_wav_waveform_summary_from_path_with_progress(
        short_path,
        &|_| {},
        &|| false,
    )
    .unwrap();
    let file =
        super::super::load_wav_waveform_summary_from_path_with_progress(path, &|_| {}, &|| false)
            .unwrap();
    assert!(file.gpu_signal_summary.levels[0].bucket_frames > 1);
    let mut state = WaveformState::from_cached_file(Arc::new(file));
    state.viewport = WaveformViewport {
        start: 1024,
        end: 1280,
    };
    let key = state
        .desired_detail_key()
        .expect("coarse overview should refine");
    state.mark_detail_pending(key.clone());

    let result = super::super::load_wav_detail_summary(key);
    assert!(result.summary.is_ok());
    state.apply_detail_result(result);

    let view = waveform_signal_surface_view(&state, None, None)
        .id(crate::native_app::test_support::waveform::WAVEFORM_SIGNAL_WIDGET_ID)
        .size(200.0, 80.0);
    let surface_view = view.into_surface();
    let bounds = Rect::from_size(200.0, 80.0);
    let layout = radiant::layout::layout_tree(&surface_view.layout_node(), bounds);
    let plan = surface_view.paint_plan(&layout, &ThemeTokens::default());
    let surface = plan.gpu_surfaces().next().unwrap();
    let GpuSurfaceContent::SignalSummaryBands {
        frames,
        frame_range,
        summary,
        ..
    } = &surface.content
    else {
        panic!("expected detail signal surface");
    };
    assert_eq!(*frames, 256);
    assert_eq!(*frame_range, [0.0, 256.0]);
    assert_eq!(summary.levels[0].bucket_frames, 2);
    assert_eq!(
        summary.levels[0].buckets, short.gpu_signal_summary.levels[0].buckets,
        "equal samples-per-pixel should produce equal detail for short and long files"
    );
    assert!(
        summary.levels[0]
            .buckets
            .iter()
            .any(|bucket| bucket.max > 0.9),
        "narrow transient in the visible long-file range should survive refinement"
    );
}

#[test]
fn waveform_detail_requests_are_single_flight_and_reject_stale_source_identity() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("stale-detail.wav");
    write_test_wav_i16(&path, &vec![1_i16; 4096]);
    let file = super::super::load_wav_waveform_summary_from_path_with_progress(
        path.clone(),
        &|_| {},
        &|| false,
    )
    .unwrap();
    let mut state = WaveformState::from_cached_file(Arc::new(file));
    state.viewport = WaveformViewport { start: 0, end: 256 };
    let key = state.desired_detail_key().unwrap();
    state.mark_detail_pending(key.clone());
    assert!(state.desired_detail_key().is_none());
    write_test_wav_i16(&path, &vec![2_i16; 4096]);

    let result = super::super::load_wav_detail_summary(key);

    assert!(
        result
            .clone()
            .summary
            .unwrap_err()
            .contains("stale waveform detail")
    );
    state.apply_detail_result(result);
    assert!(
        state.desired_detail_key().is_none(),
        "a failed identity should not create a completion/retry loop"
    );
}

#[test]
fn signal_widget_paints_gpu_surface_without_app_overlay_handles() {
    let state = WaveformState::synthetic_for_tests();
    let plan =
        waveform_signal_surface_plan(state.file(), state.viewport(), state.edit_selection(), None);

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
fn signal_widget_leaves_hover_cursor_to_waveform_widget_overlay() {
    let state = WaveformState::synthetic_for_tests();
    let plan =
        waveform_signal_surface_plan(state.file(), state.viewport(), state.edit_selection(), None);

    let surface = plan
        .gpu_surfaces()
        .find(|surface| {
            matches!(
                surface.content,
                GpuSurfaceContent::SignalSummaryBands { .. }
            )
        })
        .expect("waveform gpu surface");

    assert!(surface.capabilities.fast_pointer_move);
    assert!(surface.capabilities.coalesce_vertical_wheel);
    assert_eq!(
        surface.capabilities.runtime_overlays.pointer_vertical_line,
        None
    );
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
    let edit_selection = Some(
        wavecrate::selection::SelectionRange::new(0.2, 0.8)
            .with_fade_in(1.0, 0.0)
            .with_fade_in_mute(0.2)
            .with_fade_in_outer_gain(0.35),
    );
    let plan = waveform_signal_surface_plan(Arc::clone(&file), viewport, edit_selection, None);

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
    assert_eq!(preview.start, 0.2);
    assert_eq!(preview.end, 0.8);
    assert_eq!(preview.fade_in_length, 1.0);
    assert_eq!(preview.fade_in_curve, 0.0);
    assert_eq!(preview.fade_in_mute, 0.2);
    assert_eq!(preview.fade_in_outer_gain, 0.35);
}

#[test]
fn signal_widget_attaches_active_edit_gain_preview_without_fades() {
    let file = Arc::new(waveform_file_from_mono_samples(
        "gain-preview.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![1.0; 16],
    ));
    let viewport = super::WaveformViewport::full(file.frames);
    let edit_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.75).with_gain(0.5));
    let plan = waveform_signal_surface_plan(Arc::clone(&file), viewport, edit_selection, None);

    let surface = plan.gpu_surfaces().next().expect("waveform gpu surface");

    let GpuSurfaceContent::SignalSummaryBands { gain_preview, .. } = &surface.content else {
        panic!("expected signal summary bands");
    };
    let preview = gain_preview.expect("edit gain preview");
    assert_eq!(preview.start, 0.25);
    assert_eq!(preview.end, 0.75);
    assert_eq!(preview.gain, 0.5);
    assert_eq!(preview.fade_in_length, 0.0);
    assert_eq!(preview.fade_out_length, 0.0);
}

#[test]
fn signal_widget_attaches_sample_slide_preview_offset() {
    let state = WaveformState::synthetic_for_tests();
    let plan = waveform_signal_surface_plan(
        state.file(),
        state.viewport(),
        state.edit_selection(),
        Some(6_000),
    );

    let surface = plan.gpu_surfaces().next().expect("waveform gpu surface");
    let GpuSurfaceContent::SignalSummaryBands {
        sample_slide_frame_offset,
        ..
    } = &surface.content
    else {
        panic!("expected signal summary bands");
    };

    assert_eq!(*sample_slide_frame_offset, 6_000);
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
    let plan = waveform_signal_surface_plan(Arc::clone(&file), viewport, edit_selection, None);

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

#[test]
fn normalized_audition_signal_preview_uses_whole_sample_without_playmark() {
    let mut state = WaveformState::from_file(Arc::new(waveform_file_from_mono_samples(
        "normalized-whole.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![0.25; 16],
    )));
    Arc::make_mut(&mut state.file).playback_samples = Some(Arc::from(vec![0.25_f32; 16]));

    let preview = signal_gain_preview_for_state(&state, true).expect("normalized audition preview");

    assert_eq!(preview.start, 0.0);
    assert_eq!(preview.end, 1.0);
    assert!((preview.gain - 4.0).abs() < f32::EPSILON);
}

#[test]
fn normalized_audition_signal_preview_uses_active_playmark_peak() {
    let mut state = WaveformState::from_file(Arc::new(waveform_file_from_mono_samples(
        "normalized-playmark.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![0.25; 16],
    )));
    Arc::make_mut(&mut state.file).playback_samples = Some(Arc::from(vec![
        0.1_f32, 0.1, 0.1, 0.1, 0.25, 0.5, 0.2, 0.2, 0.9, 0.9, 0.9, 0.9, 0.1, 0.1, 0.1, 0.1,
    ]));
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.5));

    let preview = signal_gain_preview_for_state(&state, true).expect("normalized playmark preview");

    assert_eq!(preview.start, 0.25);
    assert_eq!(preview.end, 0.5);
    assert!((preview.gain - 2.0).abs() < f32::EPSILON);
}

#[test]
fn normalized_audition_signal_preview_uses_summary_when_samples_are_file_backed() {
    let root = tempfile::tempdir().expect("temp root");
    let cache_path = root.path().join("normalized-playback-cache.pcm");
    write_interleaved_f32_file(
        &cache_path,
        &[
            0.1_f32, 0.1, 0.1, 0.1, 0.25, 0.5, 0.2, 0.2, 0.9, 0.9, 0.9, 0.9, 0.1, 0.1, 0.1, 0.1,
        ],
    );
    let mut file = waveform_file_from_mono_samples(
        "normalized-cache.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![0.25; 16],
    );
    file.playback_cache_file =
        super::super::audio_file::PersistedPlaybackCacheFile::new(cache_path, 16);
    let mut state = WaveformState::from_file(Arc::new(file));
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.5));

    let playback_gain = state.normalized_audition_gain_for_span(0.25, 0.5);
    let preview = signal_gain_preview_for_state(&state, true).expect("normalized cache preview");

    assert!((playback_gain - 4.0).abs() < f32::EPSILON);
    assert_eq!(preview.start, 0.25);
    assert_eq!(preview.end, 0.5);
    assert!((preview.gain - playback_gain).abs() < f32::EPSILON);
}

#[test]
fn normalized_audition_signal_preview_uses_playmark_summary_peak_for_long_files() {
    let mut file = waveform_file_from_mono_samples(
        "normalized-long-summary.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![0.1, 0.1, 0.25, 0.5, 0.1, 0.1, 0.8, 0.8],
    );
    file.playback_samples = None;
    let mut state = WaveformState::from_file(Arc::new(file));
    state.play_selection = Some(wavecrate::selection::SelectionRange::new(0.25, 0.5));

    let playback_gain = state.normalized_audition_gain_for_span(0.25, 0.5);
    let preview = signal_gain_preview_for_state(&state, true).expect("normalized long preview");

    assert!((playback_gain - 2.0).abs() < f32::EPSILON);
    assert_eq!(preview.start, 0.25);
    assert_eq!(preview.end, 0.5);
    assert!((preview.gain - playback_gain).abs() < f32::EPSILON);
}

#[test]
fn normalized_audition_signal_preview_clears_when_disabled() {
    let mut state = WaveformState::from_file(Arc::new(waveform_file_from_mono_samples(
        "normalized-off.wav".into(),
        Arc::from([]),
        48_000,
        1,
        vec![0.25; 16],
    )));
    Arc::make_mut(&mut state.file).playback_samples = Some(Arc::from(vec![0.25_f32; 16]));

    assert!(signal_gain_preview_for_state(&state, false).is_none());
}
