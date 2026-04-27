use super::*;
#[test]
fn auto_rename_uses_db_backed_custom_tag_when_sound_type_is_missing() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    let db = controller.database_for(&source).unwrap();
    db.set_user_tag(Path::new("raw.wav"), Some("Vintage FX"))
        .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("raw.wav"), Some(128.0));
    controller.focus_browser_row_only(0);

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("artistname_SS_vintagefx_128.wav").exists());
}

#[test]
fn auto_rename_falls_back_to_numbered_identifier_when_tags_are_missing() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["untagged.wav", "untagged_001.wav", "mystery.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("untagged.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("untagged_001.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("mystery.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(2);

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("portal_SS.wav").exists());
    assert!(source.root.join("portal_SS_001.wav").exists());
}

#[test]
fn auto_rename_uses_live_sidebar_custom_tag_before_metadata_flush() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("raw.wav"), Some(128.0));
    controller.focus_browser_row_only(0);

    controller
        .apply_browser_tag_sidebar_user_tag(Some(String::from("Vintage FX")))
        .expect("custom tag should apply");
    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("portal_SS_vintagefx_128.wav").exists());
}

#[test]
fn tag_sidebar_auto_rename_renames_all_selected_paths_after_tag_change() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["raw.wav", "mystery.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("raw.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("mystery.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_tag_sidebar_auto_rename();

    controller
        .apply_browser_tag_sidebar_user_tag(Some(String::from("Vintage FX")))
        .expect("custom tag should apply and auto rename selected samples");

    assert!(source.root.join("portal_SS_vintagefx.wav").exists());
    assert!(source.root.join("portal_SS_vintagefx_001.wav").exists());
    assert!(!source.root.join("raw.wav").exists());
    assert!(!source.root.join("mystery.wav").exists());
}

#[test]
fn auto_rename_uses_live_sidebar_loop_and_sound_type_without_bpm() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .apply_browser_tag_sidebar_sound_type(Some(crate::sample_sources::SampleSoundType::Seq))
        .expect("sound type should apply");
    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("loop tag should apply");
    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("portal_loop_SEQ.wav").exists());
}

#[test]
fn auto_rename_allows_paths_with_pending_metadata_mutations() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: 1,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                blocks_file_mutation: true,
                rollback: Vec::new(),
                refresh_browser_projection: false,
            },
        );

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(!source.root.join("raw.wav").exists());
    assert!(source.root.join("portal_SS.wav").exists());
}

#[test]
fn auto_rename_ignores_pending_analysis_only_metadata_mutations() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller
        .runtime
        .source_lane
        .mutations
        .insert_metadata_mutation(
            crate::app::controller::state::runtime::PendingMetadataMutation {
                request_id: 1,
                source_id: source.id.clone(),
                paths: [PathBuf::from("raw.wav")].into_iter().collect(),
                blocks_file_mutation: false,
                rollback: Vec::new(),
                refresh_browser_projection: false,
            },
        );

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(!source.root.join("raw.wav").exists());
    assert!(source.root.join("portal_SS.wav").exists());
}

#[test]
fn auto_rename_preserves_user_tag_in_db_and_cached_entry() {
    let (mut controller, source) = dummy_controller();
    controller.settings.default_identifier = String::from("Artist Name");
    controller.ui.options_panel.default_identifier = String::from("Artist Name");
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    write_test_wav(&source.root.join("raw.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "raw.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    let db = controller.database_for(&source).unwrap();
    db.set_user_tag(Path::new("raw.wav"), Some("Vintage FX"))
        .unwrap();
    db.set_sound_type(
        Path::new("raw.wav"),
        Some(crate::sample_sources::SampleSoundType::Hat),
    )
    .unwrap();
    controller
        .ui_cache
        .browser
        .bpm_values
        .entry(source.id.clone())
        .or_default()
        .insert(PathBuf::from("raw.wav"), Some(128.0));
    controller.focus_browser_row_only(0);

    controller.auto_rename_browser_selection_action(Some(0));

    let new_relative = Path::new("artistname_SS_hat_vintagefx_128.wav");
    assert!(source.root.join(new_relative).exists());
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .user_tag_for_path(new_relative)
            .unwrap(),
        Some(String::from("Vintage FX"))
    );
    let entry_index = controller
        .wav_index_for_path(new_relative)
        .expect("renamed entry should exist in cache");
    let entry = controller
        .wav_entry(entry_index)
        .expect("renamed entry should resolve");
    assert_eq!(entry.user_tag.as_deref(), Some("Vintage FX"));
}
