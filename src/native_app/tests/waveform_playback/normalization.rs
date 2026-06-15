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

    crate::native_app::test_support::waveform::normalize_wav_file_in_place(&path)
        .expect("normalize wav");

    let samples = read_test_wav_f32(&path);
    let peak = samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    assert!((peak - 1.0).abs() < 0.000_001, "peak was {peak}");
    assert!(samples.iter().all(|sample| sample.is_finite()));

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
    assert_eq!(progress.detail, "Queued");
    assert!(state.ui.status.sample.contains("Normalizing 1 sample"));
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
            detail: selected_file.clone(),
        },
    );
    state.finish_normalization(NormalizationResult {
        task_id: 42,
        loaded_path: path.clone(),
        normalizing_loaded: true,
        was_playing: false,
        restart_ratio: 0.0,
        restart_span: None,
        normalized: vec![path.clone()],
        last_error: None,
    });

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
    let peak = state
        .waveform
        .current
        .playback_samples()
        .expect("normalized waveform should have playback samples")
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    assert!(
        (peak - 1.0).abs() < 0.000_001,
        "reloaded waveform peak should stay normalized, got {peak}"
    );

    let mut context = ui::UiUpdateContext::default();
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
    assert!(
        active_sample_load_ticket(&state).is_some(),
        "reselect should queue a fresh foreground decode instead of loading stale pre-normalized memory cache"
    );
}
