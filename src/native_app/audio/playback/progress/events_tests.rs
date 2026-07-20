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

#[test]
fn stale_progress_event_cannot_replace_the_active_requests_progress() {
    let path = PathBuf::from("request-fenced-progress.wav");
    let file =
        test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
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
    let session = state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session");
    session.runtime_request_id = Some(22);
    session.state = SamplePlaybackSessionState::WaveformVisible;
    state.audio.set_playback_progress(PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(Duration::from_millis(40)),
        looping: false,
        progress: Some(0.1),
        error: None,
    });

    state.apply_runtime_playback_progress_for_request(
        21,
        PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::from_secs(1)),
            looping: false,
            progress: Some(0.9),
            error: None,
        },
    );

    assert_eq!(state.audio.playback_progress.progress, Some(0.1));
}

#[test]
fn confirmed_retarget_becomes_active_request_and_its_terminal_progress_stops_playback() {
    let path = PathBuf::from("retarget-terminal-progress.wav");
    let file =
        test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
    state.waveform.current.start_playback(0.2);
    state.audio.current_playback_span = Some((0.2, 0.6));
    let request = SamplePlaybackRequest::waveform(
        path.display().to_string(),
        (0.2, 0.6),
        SamplePlaybackIntent::WaveformSpan,
        "waveform",
        SamplePlaybackHistory::Record,
    );
    state
        .audio
        .start_resolving_sample_playback_session(request, "decoded_samples");
    let session = state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session");
    session.runtime_request_id = Some(10);
    session.state = SamplePlaybackSessionState::WaveformVisible;
    state.audio.record_span_retarget_for_tests(11, (0.2, 0.6));

    state.apply_runtime_playback_progress_for_request(
        11,
        PlaybackRuntimeProgress {
            active: true,
            elapsed: Some(Duration::from_millis(20)),
            looping: false,
            progress: Some(0.3),
            error: None,
        },
    );
    assert_eq!(
        state
            .audio
            .sample_playback_session
            .as_ref()
            .and_then(|session| session.runtime_request_id),
        Some(11)
    );

    state.apply_runtime_playback_progress_for_request(
        11,
        PlaybackRuntimeProgress {
            active: false,
            elapsed: Some(Duration::from_millis(200)),
            looping: false,
            progress: Some(0.6),
            error: None,
        },
    );
    state.refresh_runtime_playback_progress();

    assert!(
        !state.waveform.current.is_playing(),
        "terminal progress for the confirmed retarget should leave play mode"
    );
}

#[test]
fn terminal_progress_poll_for_active_session_stops_playback() {
    let path = PathBuf::from("terminal-progress-poll.wav");
    let file =
        test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
    state.waveform.current.start_playback(0.2);
    state.audio.current_playback_span = Some((0.2, 0.6));
    let request = SamplePlaybackRequest::waveform(
        path.display().to_string(),
        (0.2, 0.6),
        SamplePlaybackIntent::WaveformSpan,
        "waveform",
        SamplePlaybackHistory::Record,
    );
    state
        .audio
        .start_resolving_sample_playback_session(request, "decoded_samples");
    let session = state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session");
    session.runtime_request_id = Some(40);
    session.state = SamplePlaybackSessionState::WaveformVisible;
    state.audio.pending_playback_progress_polls.insert(41);

    state.apply_runtime_playback_progress_for_request(
        41,
        PlaybackRuntimeProgress {
            active: false,
            elapsed: Some(Duration::from_millis(200)),
            looping: false,
            progress: Some(0.6),
            error: None,
        },
    );
    state.refresh_runtime_playback_progress();

    assert!(!state.waveform.current.is_playing());
    assert!(state.audio.pending_playback_progress_polls.is_empty());
}
