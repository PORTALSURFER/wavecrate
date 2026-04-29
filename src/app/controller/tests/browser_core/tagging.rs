use super::*;

#[test]
fn tagging_keeps_selection_on_same_sample() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
}

#[test]
fn left_tagging_from_keep_untags_then_trashes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::KEEP_1),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.tag_selected_left();
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );

    controller.tag_selected_left();
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
}

#[test]
fn tagging_under_filter_advances_focus_to_next_visible() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("four.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.set_browser_filter(TriageFlagFilter::Untagged);

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(visible_indices(&controller), vec![0, 2, 3]);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
}

#[test]
fn tagging_under_search_filter_updates_hidden_selected_paths() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.focus_browser_row_only(0);

    controller.set_browser_search(String::from("one"));
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    let one_index = controller.wav_index_for_path(Path::new("one.wav")).unwrap();
    let two_index = controller.wav_index_for_path(Path::new("two.wav")).unwrap();
    let three_index = controller
        .wav_index_for_path(Path::new("three.wav"))
        .unwrap();

    assert_eq!(
        controller.wav_entry(one_index).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
    assert_eq!(
        controller.wav_entry(two_index).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
    assert_eq!(
        controller.wav_entry(three_index).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn browser_tag_sidebar_mutation_uses_selected_visible_target_snapshot_fallback() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.ui.browser.selection.selected_visible = Some(1);
    controller.ui.browser.selection.last_focused_path = None;
    controller.ui.browser.selection.selected_paths.clear();

    controller
        .apply_browser_tag_sidebar_looped(true)
        .expect("selected-visible fallback should resolve one target");

    assert!(!controller.wav_entry(0).unwrap().looped);
    assert!(controller.wav_entry(1).unwrap().looped);
}

#[test]
fn browser_tag_sidebar_common_tag_assigns_normal_tag_catalog_entry() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.focus_browser_row_only(0);

    controller
        .apply_browser_tag_sidebar_sound_type(Some(crate::sample_sources::SampleSoundType::Kick))
        .expect("common tag should assign");

    let tags = controller
        .database_for(&source)
        .unwrap()
        .tags_for_path(Path::new("one.wav"))
        .unwrap();
    assert_eq!(tag_labels(tags), vec!["kick"]);
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .sound_type_for_path(Path::new("one.wav"))
            .unwrap(),
        None
    );
}

#[test]
fn browser_tag_sidebar_typed_input_resolves_existing_fuzzy_tag() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller
        .database_for(&source)
        .unwrap()
        .assign_tag_to_path(Path::new("one.wav"), "Deep Kick")
        .unwrap();
    controller.focus_browser_row_only(1);
    controller.set_browser_tag_sidebar_input(String::from("kick"));

    controller
        .commit_browser_tag_sidebar_input()
        .expect("typed tag should resolve and assign");

    let tags = controller
        .database_for(&source)
        .unwrap()
        .tags_for_path(Path::new("two.wav"))
        .unwrap();
    assert_eq!(tag_labels(tags), vec!["Deep Kick"]);
}

#[test]
fn browser_tag_sidebar_typed_input_creates_reusable_normal_tag() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.set_browser_tag_sidebar_input(String::from("  Vintage   FX "));

    controller
        .commit_browser_tag_sidebar_input()
        .expect("typed tag should create and assign");

    let db = controller.database_for(&source).unwrap();
    assert_eq!(
        tag_labels(db.tags_for_path(Path::new("one.wav")).unwrap()),
        vec!["Vintage FX"]
    );
    controller.focus_browser_row_only(1);
    controller.set_browser_tag_sidebar_input(String::from("vintage"));
    controller
        .commit_browser_tag_sidebar_input()
        .expect("created tag should be reusable by search");
    assert_eq!(
        tag_labels(db.tags_for_path(Path::new("two.wav")).unwrap()),
        vec!["Vintage FX"]
    );
}

#[test]
fn browser_tag_sidebar_multi_selection_tracks_mixed_and_removes_normal_tags() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller
        .database_for(&source)
        .unwrap()
        .assign_tag_to_path(Path::new("one.wav"), "kick")
        .unwrap();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    let paths = vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")];

    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "kick")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::Mixed
    );

    controller
        .apply_browser_tag_sidebar_normal_tag("kick")
        .expect("assignment should apply to every selected path");
    assert_eq!(
        controller
            .normal_tag_state_for_source(&source, &paths, "kick")
            .unwrap(),
        crate::app_core::actions::NativeBrowserTagState::On
    );
    controller
        .remove_browser_tag_sidebar_normal_tag("kick")
        .expect("removal should apply to every selected path");

    let db = controller.database_for(&source).unwrap();
    assert!(db.tags_for_path(Path::new("one.wav")).unwrap().is_empty());
    assert!(db.tags_for_path(Path::new("two.wav")).unwrap().is_empty());
}

#[test]
fn rating_filter_rating_keeps_focus_on_next_visible_item() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("four.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("four.wav"), &[0.0, 0.1]);
    controller.settings.controls.advance_after_rating = true;
    controller.settings.feature_flags.autoplay_selection = false;
    controller.set_browser_rating_filter(0, false);

    controller.focus_browser_row_only(1);
    controller.adjust_selected_rating(1);

    assert_eq!(visible_indices(&controller), vec![0, 2, 3]);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert!(browser_row_is_queued_or_loaded(
        &controller,
        Path::new("three.wav")
    ));
}

fn tag_labels(tags: Vec<crate::sample_sources::db::SourceTag>) -> Vec<String> {
    tags.into_iter().map(|tag| tag.display_label).collect()
}

#[test]
fn tagging_under_filter_uses_random_focus_in_random_mode() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    controller.settings.controls.advance_after_rating = true;
    controller.set_browser_filter(TriageFlagFilter::Untagged);
    controller.toggle_random_navigation_mode();

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(visible_indices(&controller), vec![0, 2]);
    assert_eq!(controller.history.random_history.entries.len(), 1);
    assert_eq!(controller.history.random_history.cursor, Some(0));
    let Some(selected_visible) = controller.ui.browser.selection.selected_visible else {
        panic!("expected a selected row");
    };
    assert!(selected_visible < controller.visible_browser_len());
    let selected_path = controller
        .sample_view
        .wav
        .selected_wav
        .as_deref()
        .expect("selected replacement row");
    assert!(browser_row_is_queued_or_loaded(&controller, selected_path));
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(selected_path)
    );
    assert!(controller.ui.waveform.image.is_none());
}

#[test]
fn undo_tagging_refocuses_original_sample_under_filter() {
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
    controller.set_browser_filter(TriageFlagFilter::Untagged);

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);
    assert_eq!(visible_indices(&controller), vec![0, 2]);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );

    controller.undo();

    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
    assert_eq!(
        controller.wav_entry(1).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
}

#[test]
fn direct_keep_three_tag_locks_sample_and_blocks_future_tag_changes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "keep3_direct.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller.tag_selected(crate::sample_sources::Rating::KEEP_3);

    let entry = controller
        .wav_entry(0)
        .expect("locked sample should stay loaded");
    assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_3);
    assert!(entry.locked);
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .locked_for_path(Path::new("keep3_direct.wav"))
            .unwrap(),
        Some(true)
    );

    controller.tag_selected(crate::sample_sources::Rating::NEUTRAL);

    let entry = controller
        .wav_entry(0)
        .expect("locked sample should stay loaded");
    assert_eq!(entry.tag, crate::sample_sources::Rating::KEEP_3);
    assert!(entry.locked);
}
