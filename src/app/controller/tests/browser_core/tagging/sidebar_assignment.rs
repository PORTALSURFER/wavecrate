use super::*;
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
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
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
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
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
fn browser_tag_sidebar_typed_input_commits_ordered_comma_tokens_idempotently() {
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
    controller.set_browser_tag_sidebar_input(String::from(" kick, hard, one shot, kick,,  "));

    controller
        .commit_browser_tag_sidebar_input()
        .expect("comma-delimited tags should commit");

    let tags = controller
        .database_for(&source)
        .unwrap()
        .tags_for_path(Path::new("two.wav"))
        .unwrap();
    assert_eq!(tag_labels(tags), vec!["Deep Kick", "hard", "one shot"]);
    assert_eq!(controller.ui.browser.tag_sidebar_input, "");
}
