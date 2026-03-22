use super::*;

#[test]
fn trash_move_hotkeys_are_registered() {
    let base = hotkeys::iter_actions()
        .find(|a| a.id == "move-trashed-to-folder")
        .expect("move-trashed-to-folder hotkey");
    assert_eq!(base.label, "Move trashed samples to folder");
    assert_eq!(
        base.scope,
        hotkeys::HotkeyScope::Focus(FocusContext::SampleBrowser)
    );
    assert_eq!(base.gesture.first.key, KeyCode::P);
    assert!(!base.gesture.first.shift);

    let shifted = hotkeys::iter_actions()
        .find(|a| a.id == "move-trashed-to-folder-shift")
        .expect("move-trashed-to-folder-shift hotkey");
    assert_eq!(shifted.label, "Move trashed samples to folder");
    assert_eq!(
        shifted.scope,
        hotkeys::HotkeyScope::Focus(FocusContext::SampleBrowser)
    );
    assert_eq!(shifted.gesture.first.key, KeyCode::P);
    assert!(shifted.gesture.first.shift);
}

#[test]
fn tag_neutral_hotkey_is_registered() {
    let action = hotkeys::iter_actions()
        .find(|a| a.id == "tag-neutral")
        .expect("tag-neutral hotkey");
    assert_eq!(action.label, "Neutral sample(s)");
    assert!(action.is_global());
    assert_eq!(action.gesture.first.key, KeyCode::Quote);
    assert!(!action.gesture.first.shift);
    assert!(!action.gesture.first.command);
    assert!(!action.gesture.first.alt);
    assert!(action.gesture.chord.is_none());
}

#[test]
fn quote_hotkey_tags_selected_sample_neutral() {
    let (mut controller, source) = dummy_controller();
    prepare_browser_sample(&mut controller, &source, "neutral.wav");
    controller.wav_entries.entry_mut(0).unwrap().tag = crate::sample_sources::Rating::KEEP_1;
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row(0);

    let action = hotkeys::iter_actions()
        .find(|a| a.id == "tag-neutral")
        .expect("tag-neutral hotkey");
    controller.handle_hotkey(action, FocusContext::None);

    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
}

#[test]
fn trash_move_hotkey_moves_samples() -> Result<(), String> {
    let temp = tempdir().unwrap();
    let trash_root = temp.path().join("trash");
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.settings.trash_folder = Some(trash_root.clone());
    controller.ui.trash_folder = Some(trash_root.clone());

    let trash_file = source.root.join("trash.wav");
    write_test_wav(&trash_file, &[0.1, -0.1]);

    let db = controller
        .database_for(&source)
        .map_err(|err| format!("open db: {err}"))?;
    db.upsert_file(Path::new("trash.wav"), 4, 1)
        .map_err(|err| format!("upsert: {err}"))?;
    db.set_tag(
        Path::new("trash.wav"),
        crate::sample_sources::Rating::TRASH_3,
    )
    .map_err(|err| format!("tag: {err}"))?;

    controller.set_wav_entries_for_tests(vec![sample_entry(
        "trash.wav",
        crate::sample_sources::Rating::TRASH_3,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    let action = hotkeys::iter_actions()
        .find(|a| a.id == "move-trashed-to-folder")
        .expect("move-trashed-to-folder hotkey");
    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert!(trash_root.join("trash.wav").is_file());
    assert!(!trash_file.exists());
    assert!(controller.ui.browser.trash.is_empty());
    Ok(())
}
