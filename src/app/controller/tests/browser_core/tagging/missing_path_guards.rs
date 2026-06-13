use super::*;
#[test]
/// Verifies rating write require present rejects missing path without queueing.
fn rating_write_require_present_rejects_missing_path_without_queueing() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result = controller.set_sample_tag_for_source(
        &source,
        Path::new("missing.wav"),
        crate::sample_sources::Rating::KEEP_1,
        true,
    );

    assert_eq!(result, Err(String::from("Sample not found")));
    assert!(metadata_queue_samples().is_empty());
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
}

#[test]
/// Verifies rating write without require present preserves permissive missing path behavior.
fn rating_write_without_require_present_preserves_permissive_missing_path_behavior() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result = controller.set_sample_tag_for_source(
        &source,
        Path::new("missing.wav"),
        crate::sample_sources::Rating::KEEP_1,
        false,
    );

    assert_eq!(result, Ok(()));
    assert_eq!(metadata_queue_samples().len(), 1);
}

#[test]
/// Verifies looped write require present rejects missing single path without queueing.
fn looped_write_require_present_rejects_missing_single_path_without_queueing() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result =
        controller.set_sample_looped_for_source(&source, Path::new("missing.wav"), true, true);

    assert_eq!(result, Err(String::from("Sample not found")));
    assert!(metadata_queue_samples().is_empty());
    assert!(!controller.wav_entry(0).unwrap().looped);
}

#[test]
/// Verifies looped batch require present rejects missing path before intents or cache updates.
fn looped_batch_require_present_rejects_missing_path_before_intents_or_cache_updates() {
    crate::app::controller::batch_latency::clear();
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);

    let result = controller.set_sample_looped_for_source_batch(
        &source,
        &[PathBuf::from("one.wav"), PathBuf::from("missing.wav")],
        true,
        true,
    );

    assert_eq!(result, Err(String::from("Sample not found")));
    assert!(metadata_queue_samples().is_empty());
    assert!(!controller.wav_entry(0).unwrap().looped);
}
