use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::native_app::{
    app::{
        SamplePlaybackHistory, SamplePlaybackIntent, SamplePlaybackRequest,
        SamplePlaybackSessionState, SamplePlaybackSourceProbe,
    },
    test_support::state::{NativeAppStateFixture, WaveformState},
    waveform::test_decoded_waveform_file_from_mono_samples,
};
use wavecrate::audio::PlaybackRuntimeProgress;

#[test]
fn preview_slice_runtime_progress_does_not_move_full_waveform_playhead() {
    let path = PathBuf::from("preview-visual.wav");
    let file =
        test_decoded_waveform_file_from_mono_samples(path.clone(), vec![0.0, 0.5, -0.5, 0.0]);
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
    let request = SamplePlaybackRequest::transient(
        path.display().to_string(),
        SamplePlaybackIntent::TransientNavigation,
        "browser",
    )
    .with_source_probe(SamplePlaybackSourceProbe::CachedOnly);
    state
        .audio
        .start_resolving_sample_playback_session(request, "preview_samples");
    state.audio.playback_progress = wavecrate::audio::PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(Duration::from_millis(80)),
        looping: false,
        progress: Some(0.5),
        error: None,
    };

    state.refresh_runtime_playback_progress();

    assert!(
        !state.waveform.current.is_playing(),
        "preview-slice playback should not mark the full waveform as playing"
    );
    assert_eq!(
        state.waveform.current.playhead_ratio(),
        None,
        "preview-slice progress is normalized to the tiny cache slice, not the full waveform"
    );
    assert_eq!(state.audio.current_playback_span, None);
    assert_eq!(state.audio.playback_progress.progress, Some(0.5));
}

#[test]
fn active_transient_runtime_progress_counts_as_visual_activity() {
    let mut state = NativeAppStateFixture::default().build();
    let request = SamplePlaybackRequest::transient(
        String::from("active-transient.wav"),
        SamplePlaybackIntent::TransientNavigation,
        "browser",
    );
    state
        .audio
        .start_resolving_sample_playback_session(request, "preview_samples");
    state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session")
        .state = SamplePlaybackSessionState::AudibleTransient;
    state.audio.playback_progress = PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(Duration::from_millis(90)),
        looping: false,
        progress: Some(0.5),
        error: None,
    };

    assert!(
        state.playback_visual_activity_active(),
        "active transient runtime progress should keep paint-only playback frames active"
    );
}

#[test]
fn ended_transient_runtime_progress_clears_visual_activity() {
    let mut state = NativeAppStateFixture::default().build();
    let request = SamplePlaybackRequest::transient(
        String::from("ended-transient.wav"),
        SamplePlaybackIntent::TransientNavigation,
        "browser",
    );
    state
        .audio
        .start_resolving_sample_playback_session(request, "preview_samples");
    state
        .audio
        .sample_playback_session
        .as_mut()
        .expect("sample playback session")
        .state = SamplePlaybackSessionState::AudibleTransient;
    state.audio.current_playback_span = Some((0.0, 1.0));
    state.audio.playback_progress = PlaybackRuntimeProgress {
        active: false,
        elapsed: Some(Duration::from_millis(500)),
        looping: false,
        progress: Some(1.0),
        error: None,
    };

    assert!(
        !state.playback_visual_activity_active(),
        "an inactive transient session should not keep frame work suppressed before cleanup"
    );

    state.refresh_runtime_playback_progress();

    assert_eq!(
        state.audio.sample_playback_session, None,
        "inactive transient sessions should be cleared once runtime progress reports completion"
    );
    assert_eq!(state.audio.current_playback_span, None);
    assert_eq!(
        state.audio.playback_progress,
        PlaybackRuntimeProgress::default()
    );
    assert!(!state.playback_visual_activity_active());
}

#[test]
fn pending_runtime_restart_ignores_progress_until_started_event() {
    let path = PathBuf::from("restart-visual.wav");
    let file = test_decoded_waveform_file_from_mono_samples(
        path.clone(),
        vec![0.0, 0.5, -0.5, 0.0, 0.25, -0.25],
    );
    let mut state = NativeAppStateFixture::default().build();
    state.waveform.current = WaveformState::from_cached_file(Arc::new(file));
    state.waveform.current.start_playback(0.1);
    state.audio.current_playback_span = Some((0.1, 0.9));
    let request = SamplePlaybackRequest::waveform(
        path.display().to_string(),
        (0.1, 0.9),
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
        .state = SamplePlaybackSessionState::RuntimePending;

    state.apply_runtime_playback_progress(wavecrate::audio::PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(Duration::from_millis(90)),
        looping: false,
        progress: Some(0.7),
        error: None,
    });

    assert_eq!(
        state.waveform.current.playhead_ratio(),
        Some(0.1),
        "a pending restart should keep the visual cursor at the requested start"
    );
    assert!(
        !state.audio.playback_progress.active,
        "stale runtime progress should not become the active visual clock while the new start is pending"
    );
    assert_eq!(state.audio.playback_visual_progress, None);

    state.audio.playback_progress = wavecrate::audio::PlaybackRuntimeProgress {
        active: true,
        elapsed: Some(Duration::from_millis(90)),
        looping: false,
        progress: Some(0.7),
        error: None,
    };
    state.refresh_runtime_playback_progress();

    assert_eq!(
        state.waveform.current.playhead_ratio(),
        Some(0.1),
        "frame refresh should also ignore progress snapshots until the runtime confirms the new start"
    );
}
