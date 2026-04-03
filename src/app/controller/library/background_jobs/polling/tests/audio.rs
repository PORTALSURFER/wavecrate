use super::*;
use crate::app::controller::playback::audio_loader::AudioTransientResult;
use crate::app::controller::test_support::write_test_wav;
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::sample_sources::Rating;
use std::path::Path;
use std::sync::Arc;

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
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(relative_path)
    );
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.source_id),
        Some(&source.id)
    );
}

#[test]
/// Selection audio completion should only queue one follow-loaded similarity refresh.
fn audio_primary_message_queues_one_follow_loaded_similarity_refresh() {
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("match.wav", Rating::NEUTRAL)]);
    let relative_path = Path::new("match.wav");
    write_test_wav(&source.root.join(relative_path), &[0.0, 0.25, -0.25, 0.5]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.ui.browser.search.sort = crate::app::state::SampleBrowserSort::Similarity;
    controller.ui.browser.search.similarity_sort_follow_loaded = true;

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
