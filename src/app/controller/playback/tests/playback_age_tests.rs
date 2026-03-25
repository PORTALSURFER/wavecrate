use super::super::*;
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::controller::test_support;
use std::path::{Path, PathBuf};

/// Deferred playback-age writes should remain queued until debounce expires.
#[test]
fn deferred_pending_age_update_commit_waits_for_deadline() {
    let (mut controller, source) = test_support::prepare_with_source_and_wav_entries(vec![
        test_support::sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        test_support::sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.audio.pending_age_update = Some(PendingAgeUpdate {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        played_at: 123,
    });

    controller.defer_pending_age_update_commit_if_path_changes(Path::new("two.wav"));
    assert!(controller.runtime.pending_age_update_commit.is_some());

    controller.flush_pending_age_update_commit();
    assert!(controller.runtime.pending_age_update_commit.is_some());
}

/// Expired deferred playback-age commits should persist the queued timestamp and clear the queue.
#[test]
fn flush_pending_age_update_commit_persists_last_played_after_deadline() {
    let (mut controller, source) = test_support::prepare_with_source_and_wav_entries(vec![
        test_support::sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        test_support::sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    let db = controller.database_for(&source).unwrap();
    db.upsert_file(Path::new("one.wav"), 0, 0).unwrap();
    db.upsert_file(Path::new("two.wav"), 0, 0).unwrap();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        bytes: Vec::new().into(),
        duration_seconds: 1.0,
        sample_rate: 48_000,
    });

    controller.record_loaded_audio_playback();
    let played_at = controller
        .audio
        .pending_age_update
        .as_ref()
        .map(|update| update.played_at)
        .expect("playback age update should be queued");

    controller.defer_pending_age_update_commit_if_path_changes(Path::new("two.wav"));
    controller.runtime.pending_age_update_commit_not_before =
        Some(Instant::now() - Duration::from_millis(1));

    controller.flush_pending_age_update_commit();

    assert!(!controller.has_pending_age_update_commit());
    assert!(controller.runtime.pending_age_update_commit_not_before.is_none());
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .last_played_at_for_path(Path::new("one.wav"))
            .unwrap(),
        Some(played_at)
    );
    let updated_index = controller.wav_index_for_path(Path::new("one.wav")).unwrap();
    assert_eq!(
        controller
            .wav_entries
            .entry(updated_index)
            .and_then(|entry| entry.last_played_at),
        Some(played_at)
    );
}
