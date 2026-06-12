use super::*;

#[test]
fn select_all_audio_files_selects_current_folder_samples() {
    let root = temp_source_root("wavecrate-gui-file-select-all");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    let note = drums.join("note.txt");
    fs::write(&hat, [0_u8; 8]).expect("write hat");
    fs::write(&kick, [0_u8; 8]).expect("write kick");
    fs::write(&note, [0_u8; 8]).expect("write note");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    assert_eq!(browser.select_all_audio_files(), 2);

    assert_eq!(
        browser.selected_file_paths(),
        vec![hat.clone(), kick.clone()]
    );
    assert!(!browser.is_file_selected(&path_id(&note)));

    let _ = fs::remove_dir_all(root);
}
#[test]
fn first_audio_file_path_finds_first_audio_in_selected_source_tree() {
    let root = temp_source_root("wavecrate-gui-first-startup-audio");
    let alpha = root.join("alpha");
    let beta = root.join("beta");
    fs::create_dir_all(&alpha).expect("create alpha folder");
    fs::create_dir_all(&beta).expect("create beta folder");
    fs::write(root.join("readme.txt"), []).expect("write text file");
    let first = alpha.join("a_first.wav");
    let second = beta.join("b_second.wav");
    fs::write(&first, []).expect("write first sample");
    fs::write(&second, []).expect("write second sample");

    let browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(browser.first_audio_file_path(), Some(first));
    let _ = fs::remove_dir_all(root);
}
