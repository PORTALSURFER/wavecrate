use crate::app::controller::jobs::{JobMessage, WaveformTransientResult};
use crate::app::controller::playback::audio_loader::{AudioLoadResult, AudioTransientResult};
use crate::app::controller::state::runtime::PendingWaveformTransientCompute;
use crate::app::controller::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::sample_sources::Rating;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    controller.runtime.waveform.pending_transient_compute = Some(PendingWaveformTransientCompute {
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
            .waveform
            .pending_transient_compute
            .is_none()
    );
    assert_eq!(controller.ui.waveform.transients.as_ref(), &[0.15, 0.55]);
    assert_eq!(
        controller.ui.waveform.transient_cache_token,
        Some(cache_token)
    );
}
