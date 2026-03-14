use super::*;

#[test]
fn enabling_stretch_while_playing_keeps_playing() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };

    let (mut controller, source) = dummy_controller();
    let wav_path = source.root.join("stretch_test.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);

    controller.library.sources.push(source.clone());
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));
    controller
        .load_waveform_for_selection(&source, Path::new("stretch_test.wav"))
        .expect("load waveform");
    controller.ui.waveform.bpm_value = Some(120.0);

    insert_sample_bpm(&source, Path::new("stretch_test.wav"), 80.0);

    let _ = controller.play_audio(false, None);
    if !controller.is_playing() {
        return;
    }

    controller.set_bpm_stretch_enabled(true);

    assert!(controller.is_playing());
}

#[test]
fn adjusting_bpm_while_playing_keeps_playing() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };

    let (mut controller, source) = dummy_controller();
    let wav_path = source.root.join("stretch_bpm_adjust.wav");
    let long_samples = vec![0.1_f32; 240];
    write_test_wav(&wav_path, &long_samples);

    controller.library.sources.push(source.clone());
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));
    controller
        .load_waveform_for_selection(&source, Path::new("stretch_bpm_adjust.wav"))
        .expect("load waveform");
    controller.ui.waveform.bpm_value = Some(120.0);
    insert_sample_bpm(&source, Path::new("stretch_bpm_adjust.wav"), 90.0);
    controller.set_bpm_stretch_enabled(true);

    let _ = controller.play_audio(false, None);
    if !controller.is_playing() {
        return;
    }

    controller.set_bpm_value(132.0);

    assert!(controller.is_playing());
}
