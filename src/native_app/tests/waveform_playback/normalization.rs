use super::*;
use crate::native_app::app::NormalizationResult;

#[test]
fn normalize_wav_file_in_place_scales_loaded_sample_peak() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("quiet.wav");
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    let outcome = crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect("normalize wav");
    assert_eq!(
        outcome,
        crate::native_app::test_support::waveform::WavNormalizationOutcome::Normalized
    );

    let spec = hound::WavReader::open(&path)
        .expect("open normalized wav")
        .spec();
    assert_eq!(spec.bits_per_sample, 16);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    let samples = read_test_wav_f32(&path);
    let peak = samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    assert!((peak - 1.0).abs() < 0.000_1, "peak was {peak}");
    assert!(samples.iter().all(|sample| sample.is_finite()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_wav_file_in_place_cleans_work_files_after_success() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-cleanup-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("sample.wav");
    write_test_wav_i16(&path, &[0, 2048, -4096, 8192]);

    let outcome = crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect("normalize wav");
    assert_eq!(
        outcome,
        crate::native_app::test_support::waveform::WavNormalizationOutcome::Normalized
    );

    let work_files: Vec<_> = fs::read_dir(&root)
        .expect("read temp root")
        .map(|entry| entry.expect("read temp entry").file_name())
        .filter(|file_name| file_name.to_string_lossy().contains(".wavecrate-normalize"))
        .collect();
    assert!(
        work_files.is_empty(),
        "normalization should clean temporary and backup files, found {work_files:?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_wav_file_in_place_reports_realtime_progress_phases() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-progress-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("progress.wav");
    let samples: Vec<i16> = (0..40_000)
        .map(|index| if index % 2 == 0 { 2_048 } else { -4_096 })
        .collect();
    write_test_wav_i16(&path, &samples);

    let mut snapshots = Vec::new();
    let outcome =
        crate::native_app::test_support::waveform::normalize_wav_file_in_place_with_progress(
            &path,
            |fraction, phase| snapshots.push((fraction, phase.to_string())),
        )
        .expect("normalize wav");

    assert_eq!(
        outcome,
        crate::native_app::test_support::waveform::WavNormalizationOutcome::Normalized
    );
    assert!(
        snapshots.iter().any(|(_, phase)| phase == "Analyzing"),
        "expected analyze progress updates"
    );
    assert!(
        snapshots.iter().any(|(_, phase)| phase == "Writing"),
        "expected write progress updates"
    );
    assert!(
        snapshots
            .iter()
            .any(|(fraction, _)| *fraction > 0.0 && *fraction < 1.0),
        "expected intermediate progress fractions"
    );
    assert_eq!(
        snapshots.last().expect("final progress snapshot"),
        &(1.0, String::from("Done"))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_wav_file_in_place_reports_invalid_wav_without_rewrite() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-invalid-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("truncated.wav");
    fs::write(&path, b"RIFF").expect("write truncated wav");

    let error = crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect_err("truncated wav should fail");

    assert!(
        error.starts_with("Invalid WAV:"),
        "expected invalid WAV error, got {error}"
    );
    assert_eq!(
        fs::read(&path).expect("read truncated wav after failure"),
        b"RIFF",
        "failed normalization must not rewrite invalid source files"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_selected_samples_queues_worker_without_rewriting_on_ui_thread() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("quiet.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);
    let before = fs::read(&path).expect("read wav before normalization");

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NormalizeSelectedSamples,
        &mut context,
    );

    assert_eq!(
        fs::read(&path).expect("read wav after queue"),
        before,
        "normalization must not rewrite the selected sample on the UI thread"
    );
    let progress = state
        .background
        .normalization_progress
        .as_ref()
        .expect("normalization progress should be visible after queueing");
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.total, 1);
    assert_eq!(progress.work_completed, 0);
    assert_eq!(progress.work_total, 1_000);
    assert_eq!(progress.queued, 0);
    assert_eq!(progress.detail, "Queued");
    assert!(state.ui.status.sample.contains("Normalizing 1 sample"));
}

#[test]
fn normalize_selected_samples_does_not_enqueue_duplicate_active_file() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("queued.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NormalizeSelectedSamples,
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NormalizeSelectedSamples,
        &mut context,
    );

    assert!(state.background.normalization_queue.is_empty());
    let progress = state
        .background
        .normalization_progress
        .as_ref()
        .expect("normalization progress should remain active");
    assert_eq!(progress.queued, 0);
    assert!(
        state
            .ui
            .status
            .sample
            .contains("already queued for selection")
    );
}

#[test]
fn normalize_selected_samples_uses_interactive_worker_priority() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("priority.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NormalizeSelectedSamples,
        &mut context,
    );

    assert_eq!(
        business_command_priority(context.into_command(), "gui-normalize-selected-samples"),
        Some(ui::TaskPriority::Interactive),
        "normalization must not wait behind low-priority cache warming"
    );
}

#[test]
fn uncached_sample_load_waits_for_active_normalization() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("wait-load.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);
    state.background.normalization_progress = Some(
        crate::native_app::test_support::state::NormalizationProgress {
            task_id: 77,
            label: String::from("1 sample"),
            completed: 0,
            total: 1,
            work_completed: 250,
            work_total: 1_000,
            queued: 0,
            detail: String::from("normalizing.wav"),
        },
    );

    let mut context = ui::UiUpdateContext::default();
    state.load_sample(selected_file.clone(), &mut context);

    assert!(
        active_sample_load_ticket(&state).is_none(),
        "uncached foreground decode should not start while normalization is active"
    );
    let retry_ticket = state
        .background
        .deferred_sample_load_task
        .active()
        .expect("sample load should be deferred until normalization is idle");
    assert!(state.ui.status.sample.contains("waiting for normalization"));

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket: retry_ticket,
            path: selected_file.clone(),
            autoplay: true,
            check_cache: false,
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );
    assert!(
        active_sample_load_ticket(&state).is_none(),
        "deferred retry should keep waiting while normalization is still active"
    );
    let ready_ticket = state
        .background
        .deferred_sample_load_task
        .active()
        .expect("sample load retry should be scheduled again");

    state.background.normalization_progress = None;
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket: ready_ticket,
            path: selected_file,
            autoplay: true,
            check_cache: false,
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    assert!(
        active_sample_load_ticket(&state).is_some(),
        "deferred sample load should start once normalization is idle"
    );
}

#[test]
fn normalize_wav_file_in_place_skips_already_normalized_wav() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-skip-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("full-scale.wav");
    write_test_wav_i16(&path, &[0, 32767, -32767, 1234]);
    let before = fs::read(&path).expect("read wav before normalization");

    let outcome = crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect("normalize wav");

    assert_eq!(
        outcome,
        crate::native_app::test_support::waveform::WavNormalizationOutcome::Skipped
    );
    assert_eq!(
        fs::read(&path).expect("read wav after normalization"),
        before,
        "already normalized wavs should not be rewritten"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_finish_evicts_stale_memory_cache_before_reselect() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("normalize-reselect.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("sample loads");
    let stale = state.waveform.current.clone();
    state.remember_waveform(&stale);
    assert!(state.waveform.cache.entries.contains_key(&path));

    crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect("normalize wav");
    state.background.normalization_progress = Some(
        crate::native_app::test_support::state::NormalizationProgress {
            task_id: 42,
            label: String::from("1 sample"),
            completed: 1,
            total: 1,
            work_completed: 1_000,
            work_total: 1_000,
            queued: 0,
            detail: selected_file.clone(),
        },
    );
    let mut context = ui::UiUpdateContext::default();
    state.finish_normalization(
        NormalizationResult {
            task_id: 42,
            loaded_path: path.clone(),
            normalizing_loaded: true,
            was_playing: false,
            restart_ratio: 0.0,
            restart_span: None,
            normalized: vec![path.clone()],
            skipped: Vec::new(),
            failed: Vec::new(),
        },
        &mut context,
    );

    assert!(
        !state.waveform.cache.entries.contains_key(&path),
        "normalization must evict the pre-edit memory cache entry before reselect can use it"
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&selected_file),
        "the browser loaded marker should not advertise a stale memory cache entry"
    );
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "normalization should reload the edited sample through the background load worker"
    );

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
            path: selected_file,
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "direct reselect should not use the deferred navigation load path"
    );
}

#[test]
fn normalize_finish_reloads_current_sample_without_waiting_on_queued_normalization() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("normalize-queue-reload.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("sample loads");
    crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect("normalize wav");
    state.background.normalization_progress = Some(
        crate::native_app::test_support::state::NormalizationProgress {
            task_id: 42,
            label: String::from("1 sample"),
            completed: 1,
            total: 1,
            work_completed: 1_000,
            work_total: 1_000,
            queued: 1,
            detail: selected_file.clone(),
        },
    );
    state.background.normalization_queue.push_back(
        crate::native_app::app::NormalizationQueueItem {
            paths: vec![path.clone()],
        },
    );

    let mut context = ui::UiUpdateContext::default();
    state.finish_normalization(
        NormalizationResult {
            task_id: 42,
            loaded_path: path,
            normalizing_loaded: true,
            was_playing: false,
            restart_ratio: 0.0,
            restart_span: None,
            normalized: vec![PathBuf::from(&selected_file)],
            skipped: Vec::new(),
            failed: Vec::new(),
        },
        &mut context,
    );

    assert!(
        active_sample_load_ticket(&state).is_some(),
        "the normalized current sample should reload immediately even when another normalization task is queued"
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "normalization reload must not enter the retry loop while queued normalization work exists"
    );
    assert!(
        state.background.normalization_progress.is_some(),
        "the queued normalization task should still start after scheduling the reload"
    );
}

#[test]
fn normalize_finish_resumes_loaded_playback_when_current_sample_is_skipped() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("normalize-skip-resume.wav");
    if !install_playback_runtime_for_tests(&mut state) {
        return;
    }
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 32767, -32767, 1024]);
    state
        .library
        .folder_browser
        .select_file(selected_file.clone());
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(path.clone())
            .expect("sample loads");
    state
        .start_playback_current_span(0.20, 0.80)
        .expect("sample playback starts");
    state.stop_audio_output_playback();
    state.waveform.current.stop_playback();
    state.audio.current_playback_span = None;
    state.background.normalization_progress = Some(
        crate::native_app::test_support::state::NormalizationProgress {
            task_id: 42,
            label: String::from("1 sample"),
            completed: 1,
            total: 1,
            work_completed: 1_000,
            work_total: 1_000,
            queued: 0,
            detail: selected_file.clone(),
        },
    );

    let mut context = ui::UiUpdateContext::default();
    state.finish_normalization(
        NormalizationResult {
            task_id: 42,
            loaded_path: path.clone(),
            normalizing_loaded: true,
            was_playing: true,
            restart_ratio: 0.35,
            restart_span: Some((0.20, 0.80)),
            normalized: Vec::new(),
            skipped: vec![path],
            failed: Vec::new(),
        },
        &mut context,
    );

    assert!(
        state.waveform.current.is_playing(),
        "skipped normalization should resume previously playing loaded sample"
    );
    assert_eq!(state.audio.current_playback_span, Some((0.35, 0.80)));
    assert!(
        active_sample_load_ticket(&state).is_none(),
        "skipped normalization should reuse the loaded waveform instead of reloading"
    );
}

#[test]
fn normalize_finish_reports_failed_file_without_success_count() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("normalize-failed.wav");
    let path = PathBuf::from(&selected_file);
    state.background.normalization_progress = Some(
        crate::native_app::test_support::state::NormalizationProgress {
            task_id: 42,
            label: String::from("1 sample"),
            completed: 1,
            total: 1,
            work_completed: 1_000,
            work_total: 1_000,
            queued: 0,
            detail: selected_file.clone(),
        },
    );

    let mut context = ui::UiUpdateContext::default();
    state.finish_normalization(
        NormalizationResult {
            task_id: 42,
            loaded_path: path.clone(),
            normalizing_loaded: true,
            was_playing: false,
            restart_ratio: 0.0,
            restart_span: None,
            normalized: Vec::new(),
            skipped: Vec::new(),
            failed: vec![crate::native_app::app::NormalizationFailure {
                path,
                error: String::from("Invalid WAV: Failed to read enough bytes."),
            }],
        },
        &mut context,
    );

    assert_eq!(
        state.ui.status.sample,
        "Could not normalize normalize-failed.wav | Invalid WAV: Failed to read enough bytes."
    );
    assert!(state.background.normalization_progress.is_none());
}
