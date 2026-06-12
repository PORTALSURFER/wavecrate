use super::*;

#[test]
fn file_mouse_replace_selection_clears_toggle_marked_samples() {
    let root = temp_source_root("wavecrate-gui-toggle-mark-mouse-replace");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    for file in [&hat, &kick, &snare] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&hat));
    browser
        .toggle_focused_sample_selection_and_advance(&Default::default())
        .expect("mark first sample");

    browser.select_file(path_id(&snare));

    assert_eq!(browser.selected_file_id(), Some(path_id(&snare).as_str()));
    assert_eq!(browser.selected_file_paths(), vec![snare.clone()]);

    let _ = fs::remove_dir_all(root);
}
#[test]
fn file_mouse_selection_toggles_and_extends_audio_selection() {
    let root = temp_source_root("wavecrate-gui-file-mouse-multi-select");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    let tom = drums.join("tom.wav");
    for file in [&hat, &kick, &snare, &tom] {
        fs::write(file, [0_u8; 8]).expect("write wav");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&hat));

    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![hat.clone(), snare.clone()]
    );

    browser.select_file_with_modifiers(
        path_id(&tom),
        PointerModifiers {
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![snare.clone(), tom.clone()]
    );

    browser.select_file_with_modifiers(
        path_id(&kick),
        PointerModifiers {
            command: true,
            shift: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![kick.clone(), snare.clone(), tom.clone()]
    );

    browser.select_file_with_modifiers(
        path_id(&snare),
        PointerModifiers {
            command: true,
            ..Default::default()
        },
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![kick.clone(), tom.clone()]
    );

    let _ = fs::remove_dir_all(root);
}
