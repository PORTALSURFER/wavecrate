use super::*;
use crate::app::controller::jobs::{
    FileOpResult, JobMessage, SelectionCropExportSuccess, SelectionExportAudioPayload,
    SelectionExportMessage, SelectionExportPlaybackState, SelectionExportResult,
    SelectionExportTimings, UndoFileJob, UndoFileOpResult, UndoFileOutcome,
};
use crate::app::controller::history::PendingHistoryTransactionKey;
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::test_support::write_test_wav;
use crate::app::state::{FocusContext, ProgressTaskKind, WaveformSliceBatchProfile};
use crate::app_core::state::StatusTone;
use crate::sample_sources::Rating;
use crate::waveform::{DecodedWaveform, WaveformPeaks, next_cache_token};
use hound::WavReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn pump_background_jobs_until(
    controller: &mut AppController,
    mut predicate: impl FnMut(&mut AppController) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        controller.poll_background_jobs();
        if predicate(controller) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "timed out waiting for background job condition; status='{}' tone={:?}",
        controller.ui.status.text, controller.ui.status.status_tone
    );
}

fn written_entry(root: &Path, relative_path: &Path, tag: Rating) -> WavEntry {
    let metadata = std::fs::metadata(root.join(relative_path)).expect("selection export fixture");
    let modified_ns = metadata
        .modified()
        .expect("modified time")
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("after epoch")
        .as_nanos() as i64;
    WavEntry {
        relative_path: relative_path.to_path_buf(),
        file_size: metadata.len(),
        modified_ns,
        content_hash: None,
        tag,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }
}

#[test]
fn export_selection_clip_to_root_can_flatten_name_hint() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    let clip_root = temp.path().join("export");
    std::fs::create_dir_all(source_root.join("drums")).unwrap();
    std::fs::create_dir_all(&clip_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());

    let orig = source_root.join("drums").join("clip.wav");
    write_test_wav(&orig, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("drums/clip.wav"))
        .unwrap();

    let entry = controller
        .export_selection_clip_to_root(
            SelectionClipExportRequest {
                source_id: &source.id,
                relative_path: Path::new("drums/clip.wav"),
                bounds: SelectionRange::new(0.25, 0.75),
                target_tag: None,
                add_to_browser: false,
                register_in_source: false,
            },
            &clip_root,
            Path::new("clip.wav"),
        )
        .unwrap();

    assert!(
        entry
            .relative_path
            .parent()
            .is_none_or(|p| p.as_os_str().is_empty())
    );
    assert!(clip_root.join(&entry.relative_path).is_file());
    assert!(!clip_root.join("drums").join(&entry.relative_path).exists());
}

#[test]
fn next_selection_path_in_dir_strips_existing_suffix() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("clip_selection_001.wav"), b"").unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let controller = AppController::new(renderer, None);
    let candidate =
        controller.next_selection_path_in_dir(root, Path::new("clip_selection_001.wav"));

    assert_eq!(candidate, PathBuf::from("clip_selection_002.wav"));
}

#[test]
/// Legacy `_sel` stems should still fold into the new `_selection_###` sequence.
fn next_selection_path_in_dir_strips_legacy_selection_suffix() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("clip_selection_001.wav"), b"").unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let controller = AppController::new(renderer, None);
    let candidate = controller.next_selection_path_in_dir(root, Path::new("clip_sel.wav"));

    assert_eq!(candidate, PathBuf::from("clip_selection_002.wav"));
}

#[test]
fn export_selection_clip_marks_loop_and_bpm_when_looping() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());

    let wav_path = source_root.join("looping.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("looping.wav"))
        .unwrap();
    controller.ui.waveform.loop_enabled = true;
    controller.ui.waveform.bpm_value = Some(120.0);

    let entry = controller
        .export_selection_clip(SelectionClipExportRequest {
            source_id: &source.id,
            relative_path: Path::new("looping.wav"),
            bounds: SelectionRange::new(0.0, 1.0),
            target_tag: None,
            add_to_browser: true,
            register_in_source: true,
        })
        .unwrap();

    assert!(entry.looped);
    let db = controller.database_for(&source).unwrap();
    assert_eq!(
        db.looped_for_path(&entry.relative_path).unwrap(),
        Some(true)
    );
    let conn = analysis_jobs::open_source_db(&source.root).unwrap();
    let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), &entry.relative_path);
    let bpm = analysis_jobs::sample_bpm(&conn, &sample_id).unwrap();
    assert_eq!(bpm, Some(120.0));
}

#[test]
fn export_selection_clip_applies_short_edge_fades_when_enabled() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller
        .settings
        .controls
        .auto_edge_fades_on_selection_exports = true;
    controller.ui.controls.auto_edge_fades_on_selection_exports = true;
    controller.settings.controls.anti_clip_fade_ms = 250.0;
    controller.ui.controls.anti_clip_fade_ms = 250.0;

    let wav_path = source_root.join("fades.wav");
    write_test_wav(&wav_path, &[1.0; 8]);
    controller
        .load_waveform_for_selection(&source, Path::new("fades.wav"))
        .unwrap();

    let entry = controller
        .export_selection_clip(SelectionClipExportRequest {
            source_id: &source.id,
            relative_path: Path::new("fades.wav"),
            bounds: SelectionRange::new(0.0, 1.0),
            target_tag: None,
            add_to_browser: true,
            register_in_source: true,
        })
        .unwrap();

    let target = source_root.join(&entry.relative_path);
    let mut reader = WavReader::open(&target).unwrap();
    let samples: Vec<f32> = reader.samples::<f32>().map(|s| s.unwrap()).collect();

    assert_eq!(samples.len(), 8);
    assert!(samples[0].abs() < 1e-6);
    assert!(samples[7].abs() < 1e-6);
    assert!((samples[1] - 1.0).abs() < 1e-6);
    assert!((samples[6] - 1.0).abs() < 1e-6);
}

#[test]
/// Saving from the waveform should accept deep, narrow selections on long files.
fn save_waveform_selection_to_browser_exports_narrow_deep_selection() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("long.wav");
    let samples = vec![0.25; 4096];
    write_test_wav(&wav_path, &samples);
    controller
        .load_waveform_for_selection(&source, Path::new("long.wav"))
        .unwrap();
    let narrow_deep_selection = SelectionRange::new(0.995, 0.9955);
    controller
        .selection_state
        .range
        .set_range(Some(narrow_deep_selection));
    controller.ui.waveform.selection = Some(narrow_deep_selection);

    controller
        .save_waveform_selection_to_browser(true)
        .expect("narrow selection should queue");

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("long.wav"))
    );
    pump_background_jobs_until(&mut controller, |controller| {
        source_root.join("long_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });
    assert!(source_root.join("long_selection_001.wav").is_file());
    assert!(controller.ui.status.text.contains("Saved clip"));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("long.wav"))
    );
}

#[test]
/// Queued waveform selection exports should raise one optimistic native-shell flash token.
fn save_waveform_selection_to_browser_records_flash_nonce_immediately() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("flash.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("flash.wav"))
        .unwrap();
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);

    let before = controller.ui.waveform.selection_export_flash_nonce;
    controller
        .save_waveform_selection_to_browser(true)
        .expect("selection export should queue");

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller.ui.waveform.selection_export_flash_nonce,
        before + 1
    );
    pump_background_jobs_until(&mut controller, |_| {
        source_root.join("flash_selection_001.wav").is_file()
    });

    assert_eq!(
        controller.ui.waveform.selection_export_flash_nonce,
        before + 1
    );
}

#[test]
fn save_waveform_selection_to_browser_success_finishes_pending_history_and_supports_undo() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("clip.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);

    controller
        .save_waveform_selection_to_browser(true)
        .expect("selection export should queue");

    let history_key = controller
        .history
        .pending_transactions
        .keys()
        .next()
        .cloned()
        .expect("selection export should register pending history");
    let request_id = match history_key {
        PendingHistoryTransactionKey::SelectionExport { request_id } => request_id,
        other => panic!("unexpected history key: {other:?}"),
    };
    assert_eq!(controller.history.pending_transactions.len(), 1);

    let exported_relative = PathBuf::from("clip_selection_001.wav");
    pump_background_jobs_until(&mut controller, |controller| {
        controller.history.pending_transactions.is_empty()
            && source_root.join(&exported_relative).is_file()
    });

    assert!(controller.history.pending_transactions.is_empty());
    assert!(controller.wav_index_for_path(&exported_relative).is_some());

    controller.undo();

    match controller.history.pending_undo.as_ref().map(|pending| &pending.job) {
        Some(UndoFileJob::RemoveSample {
            source_id,
            relative_path,
            ..
        }) => {
            assert_eq!(source_id, &source.id);
            assert_eq!(relative_path, &exported_relative);
        }
        other => panic!("expected deferred remove undo job, got {other:?}"),
    }
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Undoing Saved selection clip"),
        "status was {:?}",
        controller.ui.status.text
    );

    std::fs::remove_file(source_root.join(&exported_relative)).unwrap();
    controller
        .database_for(&source)
        .unwrap()
        .remove_file(&exported_relative)
        .unwrap();
    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Ok(UndoFileOutcome::Removed {
            source_id: source.id.clone(),
            relative_path: exported_relative.clone(),
        }),
        cancelled: false,
    }));

    assert!(controller.history.pending_undo.is_none());
    assert!(controller.wav_index_for_path(&exported_relative).is_none());
    assert_eq!(controller.ui.status.text, "Undid Saved selection clip");
    assert_eq!(
        PendingHistoryTransactionKey::SelectionExport { request_id },
        history_key
    );
}

#[test]
/// Failed queued waveform selection exports should raise one deferred error flash token.
fn save_waveform_selection_to_browser_records_failure_flash_when_worker_fails() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let failure_before = controller.ui.waveform.selection_export_failure_flash_nonce;
    controller.apply_background_job_message_for_tests(JobMessage::SelectionExport(
        SelectionExportMessage::Finished(SelectionExportResult::Clip {
            request_id: 99,
            result: Err(String::from("Selection export failed")),
        }),
    ));

    assert_eq!(
        controller.ui.waveform.selection_export_failure_flash_nonce,
        failure_before + 1
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Error);
    assert_eq!(controller.ui.status.text, "Selection export failed");
}

#[test]
fn selection_export_failure_cancels_pending_history_without_leaving_undo_state() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let history_key = PendingHistoryTransactionKey::SelectionExport { request_id: 99 };
    controller.begin_pending_sample_creation_transaction(history_key.clone(), "Saved selection clip");

    controller.apply_background_job_message_for_tests(JobMessage::SelectionExport(
        SelectionExportMessage::Finished(SelectionExportResult::Clip {
            request_id: 99,
            result: Err(String::from("Selection export failed")),
        }),
    ));

    assert!(controller.history.pending_transactions.is_empty());
    assert_eq!(controller.ui.status.text, "Selection export failed");

    controller.undo();

    assert!(controller.history.pending_undo.is_none());
    assert_eq!(controller.ui.status.text, "Nothing to undo");
    assert!(!controller.history.pending_transactions.contains_key(&history_key));
}

#[test]
fn apply_selection_crop_export_success_restores_focus_playback_and_undo_state() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let original_path = source_root.join("clip.wav");
    write_test_wav(&original_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller
        .set_wav_entries_for_tests(vec![written_entry(&source_root, Path::new("clip.wav"), Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.focus.context = FocusContext::SampleBrowser;

    let cropped_relative = PathBuf::from("clip_crop_001.wav");
    let cropped_absolute = source_root.join(&cropped_relative);
    write_test_wav(&cropped_absolute, &[0.2, 0.3]);
    let entry = written_entry(&source_root, &cropped_relative, Rating::KEEP_1);
    let db = controller.database_for(&source).unwrap();
    db.upsert_file(&cropped_relative, entry.file_size, entry.modified_ns)
        .unwrap();
    db.set_tag(&cropped_relative, entry.tag).unwrap();

    controller.apply_selection_crop_export_success(SelectionCropExportSuccess {
        request_id: 7,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        source_relative_path: PathBuf::from("clip.wav"),
        entry: entry.clone(),
        absolute_path: cropped_absolute,
        tag: Rating::KEEP_1,
        playback: SelectionExportPlaybackState {
            was_playing: true,
            was_looping: true,
            start_override: Some(0.25),
        },
        timings: SelectionExportTimings::default(),
    });

    assert_eq!(controller.ui.focus.context, FocusContext::Waveform);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(cropped_relative.as_path())
    );
    match controller.runtime.jobs.pending_playback.as_ref() {
        Some(pending) => {
            assert_eq!(pending.source_id, source.id);
            assert_eq!(pending.relative_path, cropped_relative);
            assert!(pending.looped);
            assert_eq!(pending.start_override, Some(0.25));
        }
        None => panic!("expected crop completion to queue playback resume"),
    }

    controller.undo();

    match controller.history.pending_undo.as_ref().map(|pending| &pending.job) {
        Some(UndoFileJob::RemoveSample {
            source_id,
            relative_path,
            ..
        }) => {
            assert_eq!(source_id, &source.id);
            assert_eq!(relative_path, &PathBuf::from("clip_crop_001.wav"));
        }
        other => panic!("expected crop undo remove job, got {other:?}"),
    }
}

#[test]
fn build_selection_export_audio_payload_prefers_loaded_decoded_samples() {
    let payload = crate::app::controller::jobs::build_selection_export_audio_payload(
        Some(&Arc::new(DecodedWaveform {
            cache_token: next_cache_token(),
            samples: Arc::from(vec![0.1, 0.2, 0.3, 0.4]),
            analysis_samples: Arc::from(Vec::<f32>::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: None,
            duration_seconds: 1.0,
            sample_rate: 44_100,
            channels: 2,
        })),
        Arc::from(vec![1u8, 2, 3]),
    );

    match payload {
        SelectionExportAudioPayload::Decoded {
            samples,
            channels,
            sample_rate,
        } => {
            assert_eq!(samples.as_ref(), &[0.1, 0.2, 0.3, 0.4]);
            assert_eq!(channels, 2);
            assert_eq!(sample_rate, 44_100);
        }
        SelectionExportAudioPayload::Encoded { .. } => {
            panic!("expected resident decoded samples to be reused");
        }
    }
}

#[test]
fn build_selection_export_audio_payload_falls_back_when_only_peak_data_is_loaded() {
    let payload = crate::app::controller::jobs::build_selection_export_audio_payload(
        Some(&Arc::new(DecodedWaveform {
            cache_token: next_cache_token(),
            samples: Arc::from(Vec::<f32>::new()),
            analysis_samples: Arc::from(Vec::<f32>::new()),
            analysis_sample_rate: 0,
            analysis_stride: 1,
            peaks: Some(Arc::new(WaveformPeaks {
                total_frames: 8,
                channels: 1,
                bucket_size_frames: 1,
                mono: vec![(0.0, 1.0)],
                left: None,
                right: None,
            })),
            duration_seconds: 1.0,
            sample_rate: 44_100,
            channels: 1,
        })),
        Arc::from(vec![1u8, 2, 3]),
    );

    match payload {
        SelectionExportAudioPayload::Encoded { bytes } => {
            assert_eq!(bytes.as_ref(), &[1, 2, 3]);
        }
        SelectionExportAudioPayload::Decoded { .. } => {
            panic!("expected peak-only waveforms to fall back to encoded bytes");
        }
    }
}

#[test]
fn save_waveform_slices_to_browser_runs_in_background_and_clears_on_success() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("clip.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.0, 0.34),
        SelectionRange::new(0.34, 0.67),
        SelectionRange::new(0.67, 1.0),
    ];
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;

    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("slice batch should queue");

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::SelectionExport)
    );
    assert!(controller.ui.progress.visible);
    assert!(!controller.ui.progress.modal);
    assert!(
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_some(),
        "slice batch should be tracked while in flight"
    );

    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
            && source_root.join("clip_silence_split_003.wav").is_file()
    });

    assert_eq!(controller.ui.status.text, "Saved 3 slices");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
    assert!(!controller.ui.progress.visible);
    assert!(controller.ui.waveform.slices.is_empty());
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn save_waveform_slices_to_browser_ignores_duplicate_submit_while_running() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("clip.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.slices = vec![SelectionRange::new(0.0, 0.5)];

    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("first slice batch should queue");
    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("duplicate submit should be ignored");

    assert_eq!(
        controller.ui.status.text,
        "Slice export already in progress"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);

    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
    });
}
