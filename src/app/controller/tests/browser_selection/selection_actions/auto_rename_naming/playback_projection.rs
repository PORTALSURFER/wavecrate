use super::*;

#[test]
fn tag_sidebar_auto_rename_preserves_active_loaded_playback_projection() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    let samples = vec![0.1_f32; 240];
    write_test_wav(&source.root.join("raw.wav"), &samples);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller
        .load_waveform_for_selection(&source, Path::new("raw.wav"))
        .expect("load waveform");
    if controller.play_audio(false, Some(0.25)).is_err() || !controller.is_playing() {
        return;
    }
    controller.ui.waveform.playhead.visible = true;
    controller.ui.waveform.playhead.position = 0.25;
    controller.focus_browser_row_only(0);
    controller.toggle_browser_tag_sidebar_auto_rename();

    controller
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("normal tag should apply and auto rename the playing sample");

    let new_relative = Path::new("portal_SS_vintagefx.wav");
    assert!(controller.is_playing());
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(new_relative)
    );
    assert_eq!(controller.ui.loaded_wav.as_deref(), Some(new_relative));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.relative_path.as_path()),
        Some(new_relative)
    );
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert!(source.root.join(new_relative).exists());
    assert!(!source.root.join("raw.wav").exists());
}
