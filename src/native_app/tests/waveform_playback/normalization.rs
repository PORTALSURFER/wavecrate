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
    assert_eq!(progress.queued, 0);
    assert_eq!(progress.detail, "Queued");
    assert!(state.ui.status.sample.contains("Normalizing 1 sample"));
}

#[test]
fn normalize_selected_samples_enqueues_when_worker_is_active() {
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

    assert_eq!(state.background.normalization_queue.len(), 1);
    let progress = state
        .background
        .normalization_progress
        .as_ref()
        .expect("normalization progress should remain active");
    assert_eq!(progress.queued, 1);
    assert!(state.ui.status.sample.contains("1 task waiting"));
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
            last_error: None,
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
