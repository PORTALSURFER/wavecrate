use super::*;

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
fn auto_rename_uses_live_sidebar_normal_tag_before_metadata_flush() {
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
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("normal tag should apply");
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
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("normal tag should apply and auto rename selected samples");

    assert!(source.root.join("portal_SS_vintagefx.wav").exists());
    assert!(source.root.join("portal_SS_vintagefx_001.wav").exists());
    assert!(!source.root.join("raw.wav").exists());
    assert!(!source.root.join("mystery.wav").exists());
}

#[test]
fn enabling_tag_sidebar_auto_rename_immediately_renames_existing_tagged_targets() {
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
        .apply_browser_tag_sidebar_normal_tag("Vintage FX")
        .expect("normal tag should apply before auto rename is enabled");
    assert!(source.root.join("raw.wav").exists());

    controller.toggle_browser_tag_sidebar_auto_rename();

    assert!(source.root.join("portal_SS_vintagefx.wav").exists());
    assert!(!source.root.join("raw.wav").exists());
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

    assert!(source.root.join("portal_loop_seq.wav").exists());
}
