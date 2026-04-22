use super::*;
#[test]
fn hotkey_tagging_applies_to_all_selected_rows() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.tag_selected_left();

    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
    assert_eq!(
        controller.wav_entry(1).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
}

#[test]
fn folder_hotkey_moves_selected_samples() {
    let (mut controller, source) = dummy_controller();
    let destination = source.root.join("dest");
    std::fs::create_dir_all(&destination).unwrap();
    controller.library.sources.push(source.clone());
    controller.select_source_by_index(0);
    controller.cache_db(&source).unwrap();
    for name in ["one.wav", "two.wav"] {
        write_test_wav(&source.root.join(name), &[0.0]);
    }
    write_test_wav(&destination.join("existing.wav"), &[0.0]);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("dest/existing.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.bind_folder_hotkey(Path::new("dest"), Some(1));
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);

    let handled = controller.apply_folder_hotkey(1, FocusContext::SampleBrowser);

    assert!(handled);
    assert!(destination.join("one.wav").exists());
    assert!(destination.join("two.wav").exists());
    assert!(!source.root.join("one.wav").exists());
    assert!(!source.root.join("two.wav").exists());
    assert!(controller
        .wav_index_for_path(&PathBuf::from("dest/one.wav"))
        .is_some());
    assert!(controller
        .wav_index_for_path(&PathBuf::from("dest/two.wav"))
        .is_some());
}

#[test]
fn update_selection_paths_rewrites_browser_selected_paths() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.ui.browser.selection.selected_paths =
        vec![PathBuf::from("old.wav"), PathBuf::from("keep.wav")];

    controller.update_selection_paths(&source, Path::new("old.wav"), Path::new("new.wav"));

    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("new.wav"), PathBuf::from("keep.wav")]
    );
}

#[test]
fn browser_action_paths_keep_hidden_selected_members() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);

    controller.set_browser_search(String::from("one"));

    assert_eq!(
        controller.browser_action_paths_from_primary(0),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn update_cached_entry_replaces_old_path_in_lookup() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "old.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.browser.selection.selected_paths = vec![PathBuf::from("old.wav")];

    let mut updated = sample_entry("new.wav", crate::sample_sources::Rating::NEUTRAL);
    updated.file_size = 10;
    updated.modified_ns = 7;
    controller.update_cached_entry(&source, Path::new("old.wav"), updated);

    assert!(controller
        .wav_index_for_path(Path::new("old.wav"))
        .is_none());
    assert!(controller
        .wav_index_for_path(Path::new("new.wav"))
        .is_some());
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("new.wav")]
    );
}

#[test]
fn select_all_populates_visible_browser_paths() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_all_browser_rows();

    assert_eq!(controller.ui.browser.selection.selected_paths.len(), 3);
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
}

#[test]
fn toggle_focused_selection_keeps_focus_on_current_row() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(1);
    controller.toggle_focused_selection();

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("two.wav")]
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn keyboard_toggle_sequence_accumulates_multi_selection() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.focus_browser_delta_action(1);
    controller.toggle_focused_selection();
    controller.focus_browser_delta_action(1);
    controller.toggle_focused_selection();

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![
            PathBuf::from("one.wav"),
            PathBuf::from("two.wav"),
            PathBuf::from("three.wav")
        ]
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("three.wav"))
    );
}

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
fn auto_rename_refuses_paths_with_pending_metadata_mutations() {
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
                rollback: Vec::new(),
                refresh_browser_projection: false,
            },
        );

    controller.auto_rename_browser_selection_action(Some(0));

    assert!(source.root.join("raw.wav").exists());
    assert!(!source.root.join("portal_SS.wav").exists());
    assert_eq!(
        controller.ui.status.text,
        "Metadata update still in progress for raw.wav; wait for it to finish before auto rename"
    );
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
