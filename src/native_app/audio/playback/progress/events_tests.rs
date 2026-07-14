use std::{path::PathBuf, sync::Arc};

use super::*;
use crate::native_app::{
    app::{SamplePlaybackHistory, SamplePlaybackIntent, SamplePlaybackRequest},
    test_support::state::{NativeAppStateFixture, WaveformState},
    waveform::test_decoded_waveform_file_from_mono_samples,
};

#[test]
fn late_original_start_preserves_newer_live_retarget_span_and_anchor() {
    let path = PathBuf::from("late-start-retarget.wav");
    let file =
        test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
    state.waveform.current.start_playback(0.0);
    state.audio.current_playback_span = Some((0.0, 1.0));
    let request = SamplePlaybackRequest::waveform(
        path.display().to_string(),
        (0.0, 1.0),
        SamplePlaybackIntent::WaveformSpan,
        "waveform",
        SamplePlaybackHistory::Record,
    );
    state
        .audio
        .start_resolving_sample_playback_session(request, "decoded_samples");
    state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session")
        .runtime_request_id = Some(1);
    state.audio.record_span_retarget_for_tests(2, (0.25, 0.60));
    state.audio.current_playback_span = Some((0.25, 0.60));
    state.audio.reset_playback_visual_progress(0.25, true);
    assert_eq!(
        state.audio.playback_visual_progress.map(|clock| clock.span),
        Some(Some((0.0, 1.0))),
        "pending requested bounds must not become the audible visual span"
    );
    state.waveform.current.start_playback(0.25);

    state.finish_runtime_playback_started_parts(1, ResolvedOutput::default(), 0.0);

    assert_eq!(state.audio.current_playback_span, Some((0.25, 0.60)));
    assert_eq!(state.audio.playback_progress.progress, Some(0.25));
    assert_eq!(state.waveform.current.playhead_ratio(), Some(0.25));
    assert!(matches!(
        state
            .audio
            .sample_playback_session
            .as_ref()
            .map(|session| &session.state),
        Some(SamplePlaybackSessionState::WaveformVisible)
    ));
}

#[test]
fn confirmed_retarget_reanchors_visual_progress_to_the_audible_span() {
    let path = PathBuf::from("confirmed-retarget.wav");
    let file =
        test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
    state.waveform.current.start_playback(0.20);
    state.audio.current_playback_span = Some((0.20, 0.60));
    let request = SamplePlaybackRequest::waveform(
        path.display().to_string(),
        (0.20, 0.60),
        SamplePlaybackIntent::WaveformSpan,
        "waveform",
        SamplePlaybackHistory::Record,
    );
    state
        .audio
        .start_resolving_sample_playback_session(request, "decoded_samples");
    state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session")
        .runtime_request_id = Some(1);
    state.finish_runtime_playback_started_parts(1, ResolvedOutput::default(), 0.20);

    state.audio.record_span_retarget_for_tests(2, (0.20, 0.80));
    state.audio.current_playback_span = Some((0.20, 0.80));
    state.audio.reset_playback_visual_progress(0.35, true);
    assert_eq!(
        state.audio.playback_visual_progress.map(|clock| clock.span),
        Some(Some((0.20, 0.60)))
    );

    assert!(state.audio.confirm_span_retarget_for_tests(2));
    state.apply_authoritative_runtime_playback_progress(PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(Duration::ZERO),
        looping: true,
        progress: Some(0.55),
        error: None,
    });

    let clock = state
        .audio
        .playback_visual_progress
        .expect("visual progress clock");
    assert_eq!(clock.span, Some((0.20, 0.80)));
    assert_eq!(clock.anchor_ratio, 0.55);
}
