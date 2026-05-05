use super::*;
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::sample_sources::Rating;

#[test]
fn enabling_loop_while_playing_restarts_in_looped_mode() {
    let Some(mut player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };

    let dir = tempdir().expect("tempdir");
    let wav_path = dir.path().join("loop_test.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);
    let bytes = std::fs::read(&wav_path).expect("wav bytes");
    player.set_audio(bytes, 30.0);
    player.play_range(0.0, 1.0, false).expect("play range");

    let (mut controller, source) = dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("loop_test.wav"),
        bytes: std::fs::read(&wav_path).expect("wav bytes").into(),
        duration_seconds: 30.0,
        sample_rate: 8,
    });
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));

    controller.ui.waveform.loop_enabled = false;
    if !controller.is_playing() {
        return;
    }

    controller.toggle_loop();

    assert!(controller.ui.waveform.loop_enabled);
    assert!(
        controller
            .audio
            .player
            .as_ref()
            .expect("player")
            .borrow()
            .is_looping()
    );
}

#[test]
fn enabling_loop_while_playing_uses_full_selection() {
    let Some(mut player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };

    let dir = tempdir().expect("tempdir");
    let wav_path = dir.path().join("loop_selection_test.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);
    let bytes = std::fs::read(&wav_path).expect("wav bytes");
    let duration = 30.0;
    player.set_audio(bytes, duration);
    player.play_range(0.0, 1.0, false).expect("play range");

    let (mut controller, source) = dummy_controller();
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("loop_selection_test.wav"),
        bytes: std::fs::read(&wav_path).expect("wav bytes").into(),
        duration_seconds: duration,
        sample_rate: 8,
    });
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));

    if !controller.is_playing() {
        return;
    }

    let selection = SelectionRange::new(0.2, 0.6);
    controller.selection_state.range.set_range(Some(selection));
    controller.apply_selection(Some(selection));

    controller.ui.waveform.loop_enabled = false;
    controller.toggle_loop();

    let (start, end) = controller
        .audio
        .player
        .as_ref()
        .expect("player")
        .borrow()
        .play_span()
        .expect("play span set");
    let expected_start = duration * selection.start();
    let expected_end = duration * selection.end();
    assert!((start - expected_start).abs() < 1e-4);
    assert!((end - expected_end).abs() < 1e-4);
}

#[test]
fn toggle_loop_persists_hidden_selected_paths() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    let db = controller.cache_db(&source).expect("db");
    for name in ["one.wav", "two.wav", "three.wav"] {
        write_test_wav(&source.root.join(name), &[0.0, 0.1]);
        db.upsert_file(Path::new(name), 4, 1)
            .expect("seed source db row");
        db.set_looped(Path::new(name), false)
            .expect("seed loop marker");
    }

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.sample_view.wav.loaded_audio = Some(LoadedAudio {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        bytes: std::fs::read(source.root.join("one.wav"))
            .expect("wav bytes")
            .into(),
        duration_seconds: 1.0,
        sample_rate: 8,
    });
    controller.ui.waveform.bpm_value = Some(120.0);

    controller.set_browser_search(String::from("one"));
    controller.toggle_loop();

    assert_eq!(
        db.looped_for_path(Path::new("one.wav"))
            .expect("load one loop marker"),
        Some(true)
    );
    assert_eq!(
        db.looped_for_path(Path::new("two.wav"))
            .expect("load two loop marker"),
        Some(true)
    );
    assert_eq!(
        db.looped_for_path(Path::new("three.wav"))
            .expect("load three loop marker"),
        Some(false)
    );
}
