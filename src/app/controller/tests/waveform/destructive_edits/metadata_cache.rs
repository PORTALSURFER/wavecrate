use super::*;

#[test]
fn destructive_edit_preserves_cached_browser_metadata() {
    let mut entry = sample_entry("rich.wav", crate::sample_sources::Rating::KEEP_3);
    entry.looped = true;
    entry.locked = true;
    entry.sound_type = Some(SampleSoundType::Kick);
    entry.user_tag = Some(String::from("Vintage FX"));
    entry.last_played_at = Some(1_234);
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![entry]);
    let wav_path = source.root.join("rich.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    let db = controller.database_for(&source).unwrap();
    db.set_looped(Path::new("rich.wav"), true).unwrap();
    db.set_locked(Path::new("rich.wav"), true).unwrap();
    db.set_sound_type(Path::new("rich.wav"), Some(SampleSoundType::Kick))
        .unwrap();
    db.set_user_tag(Path::new("rich.wav"), Some("Vintage FX"))
        .unwrap();
    db.assign_tag_to_path(Path::new("rich.wav"), "kick")
        .unwrap();
    db.assign_tag_to_path(Path::new("rich.wav"), "Vintage FX")
        .unwrap();
    db.set_last_played_at(Path::new("rich.wav"), 1_234).unwrap();
    controller
        .load_waveform_for_selection(&source, Path::new("rich.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.25, 0.75));

    controller.crop_waveform_selection().unwrap();

    let cached = controller.wav_entry(0).unwrap().clone();
    let persisted = db.entry_for_path(Path::new("rich.wav")).unwrap().unwrap();
    for row in [&cached, &persisted] {
        assert_eq!(row.tag, crate::sample_sources::Rating::KEEP_3);
        assert!(row.looped);
        assert!(row.locked);
        assert_eq!(row.sound_type, Some(SampleSoundType::Kick));
        assert_eq!(row.user_tag.as_deref(), Some("Vintage FX"));
        assert_eq!(row.last_played_at, Some(1_234));
    }
    controller.focus_browser_row_only(0);
    controller.ui.browser.tag_sidebar_open = true;
    let projected = project_browser_model(&mut controller);
    assert!(projected.rows[0].locked);
    assert_eq!(projected.rows[0].bucket_label.as_deref(), Some("LOOP"));
    assert_eq!(
        projected
            .tag_sidebar
            .option_pills
            .iter()
            .find(|pill| pill.label == "kick")
            .map(|pill| pill.state),
        Some(crate::app_core::actions::NativeBrowserTagState::On)
    );
    assert_eq!(
        projected
            .tag_sidebar
            .option_pills
            .iter()
            .find(|pill| pill.label == "Vintage FX")
            .map(|pill| pill.state),
        Some(crate::app_core::actions::NativeBrowserTagState::On)
    );
}

#[test]
fn destructive_edit_clears_stale_content_hash() {
    let mut entry = sample_entry("hash.wav", crate::sample_sources::Rating::KEEP_1);
    entry.content_hash = Some(String::from("old-content-hash"));
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![entry]);
    let wav_path = source.root.join("hash.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    let db = controller.database_for(&source).unwrap();
    let mut batch = db.write_batch().unwrap();
    batch
        .upsert_file_with_hash_and_tag(
            Path::new("hash.wav"),
            4,
            1,
            "old-content-hash",
            crate::sample_sources::Rating::KEEP_1,
            false,
        )
        .unwrap();
    batch.commit().unwrap();
    controller
        .load_waveform_for_selection(&source, Path::new("hash.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.25, 0.75));

    controller.crop_waveform_selection().unwrap();

    let cached = controller.wav_entry(0).unwrap();
    let persisted = db.entry_for_path(Path::new("hash.wav")).unwrap().unwrap();
    assert_eq!(cached.content_hash, None);
    assert_eq!(persisted.content_hash, None);
    assert_eq!(persisted.tag, crate::sample_sources::Rating::KEEP_1);
}
