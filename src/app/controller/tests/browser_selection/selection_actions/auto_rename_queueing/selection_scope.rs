use super::*;

#[test]
fn auto_rename_uses_primary_row_plus_hidden_selection() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["kick.wav", "bass.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("kick.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("bass.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    let db = controller.database_for(&source).unwrap();
    db.set_sound_type(
        Path::new("kick.wav"),
        Some(crate::sample_sources::SampleSoundType::Kick),
    )
    .unwrap();
    db.set_sound_type(
        Path::new("bass.wav"),
        Some(crate::sample_sources::SampleSoundType::Bass),
    )
    .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("kick.wav"), Some(130.0));
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("bass.wav"), Some(131.0));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.set_browser_search(String::from("kick"));
    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("artistname_SS_kick_130.wav").exists());
    assert!(source.root.join("artistname_SS_bass_131.wav").exists());
}
