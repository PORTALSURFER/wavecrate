use super::*;
use crate::app::controller::jobs::WaveformTransientResult;
use crate::app::controller::playback::audio_loader::{AudioTransientResult, AudioVisualResult};
use crate::app::controller::state::runtime::PendingWaveformTransientCompute;
use crate::app::controller::test_support::write_test_wav;
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::sample_sources::Rating;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
/// Primary audio completions should ignore stale requests and keep loading active until visuals arrive.
fn audio_primary_message_ignores_stale_completion_then_applies_matching_result() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("match.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("match.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller.ui.waveform.loading = Some(relative_path.to_path_buf());
    controller
        .runtime
        .jobs
        .set_pending_audio(Some(PendingAudio {
            request_id: 17,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            intent: AudioLoadIntent::Selection,
        }));

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Primary {
            request_id: 18,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            result: Ok(decode_audio_outcome(&controller, &source, relative_path)),
        },
    ));

    let pending = controller.runtime.jobs.pending_audio();
    assert!(pending.is_some(), "stale completion should stay pending");
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(relative_path)
    );
    assert!(controller.sample_view.wav.loaded_audio.is_none());
    assert!(controller.runtime.jobs.staged_audio_handoff().is_none());

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Primary {
            request_id: 17,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            result: Ok(decode_audio_outcome(&controller, &source, relative_path)),
        },
    ));

    assert!(controller.runtime.jobs.pending_audio().is_none());
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(relative_path)
    );
    assert!(controller.sample_view.wav.loaded_wav.is_none());
    let staged = controller
        .runtime
        .jobs
        .staged_audio_handoff()
        .expect("matching primary completion should stage the handoff");
    assert_eq!(staged.source_id, source.id);
    assert_eq!(staged.relative_path, relative_path);
}

#[test]
/// Matching visual completion should publish the new sample and resolve pending playback together.
fn audio_visual_message_commits_staged_handoff_before_playback() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("match.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("match.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller.ui.waveform.loading = Some(relative_path.to_path_buf());
    controller
        .runtime
        .jobs
        .set_pending_playback(Some(PendingPlayback {
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            looped: false,
            start_override: None,
            force_loaded_audio: false,
        }));

    let outcome = decode_audio_outcome(&controller, &source, relative_path);
    let cache_token = outcome.decoded.cache_token;
    controller.handle_audio_loaded(
        PendingAudio {
            request_id: 17,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            intent: AudioLoadIntent::Selection,
        },
        outcome,
    );
    assert!(controller.sample_view.wav.loaded_wav.is_none());
    assert!(!controller.is_playing());
    assert!(controller.runtime.jobs.pending_playback.is_some());

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Visual(AudioVisualResult {
            request_id: 17,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            metadata: controller
                .current_file_metadata(&source, relative_path)
                .expect("metadata"),
            cache_token,
            transients: Arc::from(vec![0.2, 0.7]),
            image: None,
            projected_image: None,
            render_meta: None,
            stretched: false,
        }),
    ));

    assert!(controller.runtime.jobs.staged_audio_handoff().is_none());
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(relative_path)
    );
    assert_eq!(
        controller.ui.loaded_wav.as_deref(),
        Some(relative_path)
    );
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.relative_path.as_path()),
        Some(relative_path)
    );
}

#[test]
/// Selection handoff should queue one follow-loaded similarity refresh only after visuals commit.
fn audio_visual_message_queues_one_follow_loaded_similarity_refresh() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("match.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("match.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.ui.browser.search.sort = crate::app::state::SampleBrowserSort::Similarity;
    controller.ui.browser.search.similarity_sort_follow_loaded = true;
    controller.ui.waveform.loading = Some(relative_path.to_path_buf());

    controller.handle_audio_loaded(
        PendingAudio {
            request_id: 17,
            source_id: source.id.clone(),
            root: source.root.clone(),
            relative_path: relative_path.to_path_buf(),
            intent: AudioLoadIntent::Selection,
        },
        decode_audio_outcome(&controller, &source, relative_path),
    );
    assert!(controller.runtime.pending_loaded_similarity_query.is_none());

    let staged = controller
        .runtime
        .jobs
        .staged_audio_handoff()
        .expect("primary completion should stage the handoff");
    controller.handle_audio_visual_loaded(AudioVisualResult {
        request_id: staged.request_id,
        source_id: source.id.clone(),
        relative_path: relative_path.to_path_buf(),
        metadata: controller
            .current_file_metadata(&source, relative_path)
            .expect("metadata"),
        cache_token: staged.decoded.cache_token,
        transients: Arc::from(vec![0.2, 0.7]),
        image: None,
        projected_image: None,
        render_meta: None,
        stretched: false,
    });

    let pending = controller
        .runtime
        .pending_loaded_similarity_query
        .as_ref()
        .expect("follow-loaded similarity query should be queued");
    assert_eq!(pending.request_id, 1);
    assert_eq!(pending.source_id, source.id);
    assert_eq!(pending.relative_path, relative_path);
}

#[test]
/// Transient completions should route through the controller and refresh the active waveform UI.
fn audio_transients_message_routes_to_loaded_waveform_state() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("route.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("route.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller
        .load_waveform_for_selection(&source, relative_path)
        .expect("initial waveform load");
    let metadata = controller
        .current_file_metadata(&source, relative_path)
        .expect("metadata");
    let cache_token = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .cache_token;
    controller.ui.waveform.transients = Arc::from([]);
    controller.ui.waveform.transient_cache_token = None;

    controller.apply_background_job_message_for_tests(JobMessage::AudioLoaded(
        AudioLoadResult::Transients(AudioTransientResult {
            request_id: 17,
            source_id: source.id.clone(),
            relative_path: relative_path.to_path_buf(),
            metadata,
            cache_token,
            transients: Arc::from(vec![0.2, 0.7]),
            stretched: true,
        }),
    ));

    assert_eq!(controller.ui.waveform.transients.as_ref(), &[0.2, 0.7]);
    assert_eq!(
        controller.ui.waveform.transient_cache_token,
        Some(cache_token)
    );
}

#[test]
/// Deferred waveform-transient completions should apply only to the active pending request.
fn waveform_transients_computed_message_routes_to_loaded_waveform_state() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("deferred.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("deferred.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller
        .load_waveform_for_selection(&source, relative_path)
        .expect("initial waveform load");
    let cache_token = controller
        .sample_view
        .waveform
        .decoded
        .as_ref()
        .expect("decoded waveform")
        .cache_token;
    controller.runtime.pending_waveform_transient_compute = Some(PendingWaveformTransientCompute {
        request_id: 17,
        cache_token,
        queued_at: Instant::now(),
    });
    controller.ui.waveform.transients = Arc::from([]);
    controller.ui.waveform.transient_cache_token = None;

    controller.apply_background_job_message_for_tests(JobMessage::WaveformTransientsComputed(
        WaveformTransientResult {
            request_id: 17,
            cache_token,
            elapsed: Duration::from_millis(4),
            result: Ok(Arc::from(vec![0.15, 0.55])),
        },
    ));

    assert!(
        controller
            .runtime
            .pending_waveform_transient_compute
            .is_none()
    );
    assert_eq!(controller.ui.waveform.transients.as_ref(), &[0.15, 0.55]);
    assert_eq!(
        controller.ui.waveform.transient_cache_token,
        Some(cache_token)
    );
}
