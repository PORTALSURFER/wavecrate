use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use crate::native_app::waveform::{
    InstantWaveformPreview, InstantWaveformPreviewTier, PreviewAuditionClip, WaveformState,
};

use super::super::WaveformAppState;
use super::*;

fn preview_clip(path: PathBuf) -> PreviewAuditionClip {
    PreviewAuditionClip {
        path,
        source_len: 0,
        source_modified: Some(SystemTime::UNIX_EPOCH),
        samples: Arc::from([0.0_f32]),
        sample_rate: 44_100,
        channels: 1,
        frames: 1,
        normalized_gain: 1.0,
    }
}

fn instant_preview(path: PathBuf, bucket_count: usize) -> InstantWaveformPreview {
    let file = crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
        path,
        vec![0.25; bucket_count.max(1)],
    );
    InstantWaveformPreview {
        file: Arc::new(file),
        tier: InstantWaveformPreviewTier::Head,
        source_len: 0,
        source_modified: Some(SystemTime::UNIX_EPOCH),
    }
}

#[test]
fn preview_warm_attempt_marker_survives_missing_clip_probe() {
    let path = PathBuf::from("/tmp/wavecrate-preview-missing.wav");
    let path_id = path.display().to_string();
    let mut cache = WaveformCacheState::default();

    assert!(cache.preview_audition_warm_needed(&path));
    cache.finish_preview_audition_warm_schedule(
        std::slice::from_ref(&path_id),
        std::slice::from_ref(&path_id),
        std::slice::from_ref(&path_id),
    );
    assert!(!cache.preview_audition_warm_needed(&path));
    assert_eq!(cache.preview_audition_clip(&path), None);
    assert!(
        !cache.preview_audition_warm_needed(&path),
        "a failed warm attempt must not become eligible again on the next frame"
    );
}

#[test]
fn preview_clip_lookup_trusts_cached_head_without_file_metadata_probe() {
    let path = PathBuf::from("/tmp/wavecrate-preview-head-without-source-file.wav");
    let mut cache = WaveformCacheState::default();
    cache.store_preview_audition_clip(preview_clip(path.clone()));

    let clip = cache.preview_audition_clip(&path);

    assert!(
        clip.is_some(),
        "hot preview-head playback lookup should not require a synchronous filesystem metadata probe"
    );
    assert!(
        cache.preview_audition_clips.contains_key(&path),
        "hot lookup should not evict a cached preview head just because the source file is unavailable during the UI update"
    );
}

#[test]
fn instant_waveform_preview_lookup_trusts_cache_without_file_metadata_probe() {
    let path = PathBuf::from("/tmp/wavecrate-missing-instant-waveform-preview.wav");
    let mut cache = WaveformCacheState::default();
    cache.store_instant_waveform_preview(instant_preview(path.clone(), 4));

    let preview = cache.instant_waveform_preview(&path);

    assert!(
        preview.is_some(),
        "hot starmap visual lookup should not probe source metadata"
    );
    assert!(
        cache.instant_waveform_previews.contains_key(&path),
        "hot lookup should not evict a cached visual preview just because the source path is unavailable"
    );
}

#[test]
fn instant_waveform_preview_cache_prunes_oldest_entries() {
    let first = PathBuf::from("/tmp/wavecrate-preview-first.wav");
    let second = PathBuf::from("/tmp/wavecrate-preview-second.wav");
    let third = PathBuf::from("/tmp/wavecrate-preview-third.wav");
    let mut cache = WaveformCacheState::default();

    cache.store_instant_waveform_preview(instant_preview(first.clone(), 8));
    cache.store_instant_waveform_preview(instant_preview(second.clone(), 8));
    let _ = cache.instant_waveform_preview(&second);
    cache.store_instant_waveform_preview(instant_preview(third.clone(), 8));

    assert!(
        !cache.instant_waveform_previews.contains_key(&first),
        "oldest visual preview should be pruned first"
    );
    assert!(
        cache.instant_waveform_previews.contains_key(&third),
        "newest visual preview should be retained"
    );
}

#[test]
fn instant_waveform_preview_loading_replaces_stale_waveform() {
    let mut app_state = WaveformAppState::new(WaveformState::from_cached_file(Arc::new(
        crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
            PathBuf::from("/tmp/old.wav"),
            vec![0.0, 0.2, -0.2],
        ),
    )));

    let previous = app_state
        .replace_current_with_instant_waveform_preview_loading(PathBuf::from("/tmp/new.wav"));

    assert_eq!(previous.path(), PathBuf::from("/tmp/old.wav"));
    assert!(!app_state.current.has_loaded_sample());
    assert_eq!(
        app_state.instant_preview_path(),
        Some(Path::new("/tmp/new.wav"))
    );
}

#[test]
fn playback_handoff_rebinds_visible_waveform_to_new_sample_preview() {
    let old_path = PathBuf::from("/tmp/handoff-old.wav");
    let new_path = PathBuf::from("/tmp/handoff-new.wav");
    let mut app_state = WaveformAppState::new(WaveformState::from_cached_file(Arc::new(
        crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
            old_path.clone(),
            vec![0.0, 0.2, -0.2],
        ),
    )));

    let previous = app_state
        .begin_playback_visual_handoff(new_path.clone(), Some(instant_preview(new_path.clone(), 8)))
        .expect("cross-sample handoff");

    assert_eq!(app_state.current.path(), new_path);
    assert_eq!(
        app_state.instant_preview_path(),
        Some(Path::new("/tmp/handoff-new.wav"))
    );
    let discarded = app_state.rollback_playback_visual_handoff(previous);
    assert_eq!(app_state.current.path(), old_path);
    assert_eq!(discarded.path(), new_path);
}

#[test]
fn playback_handoff_replaces_same_path_waveform_during_a_fresh_load() {
    let path = PathBuf::from("/tmp/handoff-overwritten.wav");
    let mut app_state = WaveformAppState::new(WaveformState::from_cached_file(Arc::new(
        crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
            path.clone(),
            vec![0.0, 0.2, -0.2],
        ),
    )));

    let previous = app_state
        .begin_playback_visual_handoff(path.clone(), Some(instant_preview(path.clone(), 8)))
        .expect("fresh same-path load should replace stale visuals");

    assert_eq!(app_state.current.path(), path);
    assert!(app_state.instant_preview_active());
    let discarded = app_state.rollback_playback_visual_handoff(previous);
    assert_eq!(discarded.path(), path);
    assert!(!app_state.instant_preview_active());
}

#[test]
fn playback_handoff_blanks_old_waveform_without_new_sample_preview() {
    let mut app_state = WaveformAppState::new(WaveformState::from_cached_file(Arc::new(
        crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
            PathBuf::from("/tmp/handoff-old.wav"),
            vec![0.0, 0.2, -0.2],
        ),
    )));
    app_state.load.label = Some(String::from("handoff-old.wav"));

    let previous = app_state
        .begin_playback_visual_handoff(PathBuf::from("/tmp/handoff-new.wav"), None)
        .expect("cross-sample handoff");

    assert!(!app_state.current.has_loaded_sample());
    assert_eq!(
        app_state.instant_preview_path(),
        Some(Path::new("/tmp/handoff-new.wav"))
    );
    let discarded = app_state.rollback_playback_visual_handoff(previous);
    assert_eq!(app_state.current.path(), Path::new("/tmp/handoff-old.wav"));
    assert_eq!(app_state.load.label.as_deref(), Some("handoff-old.wav"));
    assert!(!discarded.has_loaded_sample());
}

#[test]
fn successful_playback_handoff_preserves_starmap_drag_restore() {
    let old_path = PathBuf::from("/tmp/starmap-handoff-old.wav");
    let new_path = PathBuf::from("/tmp/starmap-handoff-new.wav");
    let mut app_state = WaveformAppState::new(WaveformState::from_cached_file(Arc::new(
        crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
            old_path.clone(),
            vec![0.0, 0.2, -0.2],
        ),
    )));
    app_state.capture_starmap_drag_restore();

    let snapshot = app_state
        .begin_playback_visual_handoff(new_path.clone(), Some(instant_preview(new_path, 8)))
        .expect("cross-sample handoff");
    drop(snapshot);

    assert!(app_state.starmap_drag_restore.is_some());
    let discarded = app_state
        .restore_starmap_drag_snapshot()
        .expect("mouse-up should still restore the pre-drag waveform");
    assert_eq!(app_state.current.path(), old_path);
    assert_ne!(discarded.path(), old_path);
}

#[test]
fn failed_full_load_clears_committed_instant_preview() {
    let old_path = PathBuf::from("/tmp/failed-handoff-old.wav");
    let new_path = PathBuf::from("/tmp/failed-handoff-new.wav");
    let mut app_state = WaveformAppState::new(WaveformState::from_cached_file(Arc::new(
        crate::native_app::waveform::test_file_backed_waveform_file_from_mono_samples(
            old_path.clone(),
            vec![0.0, 0.2, -0.2],
        ),
    )));
    app_state.capture_starmap_drag_restore();
    let snapshot = app_state
        .begin_playback_visual_handoff(new_path.clone(), Some(instant_preview(new_path.clone(), 8)))
        .expect("cross-sample handoff");
    drop(snapshot);

    let discarded = app_state
        .clear_failed_instant_preview(&new_path)
        .expect("failed target preview should be cleared");

    assert_eq!(app_state.current.path(), old_path);
    assert_eq!(discarded.path(), new_path);
    assert!(!app_state.instant_preview_active());
}

#[test]
fn preview_warm_scheduled_path_is_not_requeued_before_completion() {
    let path = PathBuf::from("/tmp/wavecrate-preview-scheduled.wav");
    let path_id = path.display().to_string();
    let mut cache = WaveformCacheState::default();

    assert!(cache.preview_audition_warm_needed(&path));
    cache.mark_preview_audition_warm_scheduled(std::slice::from_ref(&path_id));
    assert!(
        !cache.preview_audition_warm_needed(&path),
        "a scheduled warm path must not be rediscovered by the next frame"
    );

    cache.finish_preview_audition_warm_schedule(
        std::slice::from_ref(&path_id),
        std::slice::from_ref(&path_id),
        &[],
    );
    assert!(
        !cache.preview_audition_warm_needed(&path),
        "a completed warm attempt should stay ineligible even if no clip was produced"
    );
}

#[test]
fn preview_warm_cancel_releases_scheduled_paths_for_later_warm() {
    let path = PathBuf::from("/tmp/wavecrate-preview-cancelled.wav");
    let path_id = path.display().to_string();
    let mut cache = WaveformCacheState::default();

    cache.mark_preview_audition_warm_scheduled(std::slice::from_ref(&path_id));
    assert!(!cache.preview_audition_warm_needed(&path));
    cache.cancel_preview_audition_warm_schedule();

    assert!(
        cache.preview_audition_warm_needed(&path),
        "cancelled background warm work should be eligible once the UI is idle again"
    );
}

#[test]
fn preview_warm_partial_finish_releases_unattempted_tail_for_later_warm() {
    let attempted = PathBuf::from("/tmp/wavecrate-preview-attempted.wav");
    let skipped = PathBuf::from("/tmp/wavecrate-preview-skipped.wav");
    let attempted_id = attempted.display().to_string();
    let skipped_id = skipped.display().to_string();
    let mut cache = WaveformCacheState::default();

    cache.mark_preview_audition_warm_scheduled(&[attempted_id.clone(), skipped_id.clone()]);
    cache.finish_preview_audition_warm_schedule(
        &[attempted_id, skipped_id],
        &[attempted.display().to_string()],
        &[],
    );

    assert!(!cache.preview_audition_warm_needed(&attempted));
    assert!(
        cache.preview_audition_warm_needed(&skipped),
        "scheduled-but-unattempted warm tails should be retried after the partial worker exits"
    );
    assert!(
        cache.preview_audition_decode_needed(&skipped),
        "foreground drag playback can still decode a path skipped by background warming"
    );
}

#[test]
fn confirmed_preview_failure_is_not_retried_by_foreground_decode() {
    let path = PathBuf::from("/tmp/wavecrate-preview-failed.wav");
    let mut cache = WaveformCacheState::default();

    assert!(cache.preview_audition_decode_needed(&path));
    cache.mark_preview_audition_failed(&path);

    assert!(
        !cache.preview_audition_decode_needed(&path),
        "interactive preview decode should not churn on a path that already failed preview decoding"
    );
    assert!(
        !cache.preview_audition_warm_needed(&path),
        "confirmed preview failures should remain out of background warm planning"
    );
}

#[test]
fn warm_failed_path_is_not_retried_by_foreground_decode() {
    let path = PathBuf::from("/tmp/wavecrate-preview-warm-failed.wav");
    let path_id = path.display().to_string();
    let mut cache = WaveformCacheState::default();

    cache.mark_preview_audition_warm_scheduled(std::slice::from_ref(&path_id));
    cache.finish_preview_audition_warm_schedule(
        std::slice::from_ref(&path_id),
        std::slice::from_ref(&path_id),
        std::slice::from_ref(&path_id),
    );

    assert!(
        !cache.preview_audition_decode_needed(&path),
        "foreground drag/list/keyboard playback should skip known failed preview heads"
    );
}

#[test]
fn preview_head_cache_does_not_mark_sample_fully_instant_ready() {
    let path = PathBuf::from("/tmp/wavecrate-preview-head.wav");
    let path_id = path.display().to_string();
    let mut cache = WaveformCacheState::default();

    cache.store_preview_audition_clip(preview_clip(path.clone()));

    assert!(cache.preview_audition_clips.contains_key(&path));
    assert!(
        !cache.instant_audition_sample_paths.contains(&path_id),
        "a tiny preview head should not make the UI advertise full instant-audition readiness"
    );
}

#[test]
fn preview_cache_eviction_does_not_immediately_requeue_warm() {
    let path = PathBuf::from("/tmp/wavecrate-preview-evicted.wav");
    let mut cache = WaveformCacheState::default();

    cache.store_preview_audition_clip(preview_clip(path.clone()));
    cache.evict_preview_audition_clip(&path);

    assert!(
        !cache.preview_audition_warm_needed(&path),
        "background preview warm should not churn on evicted cache entries"
    );
}
