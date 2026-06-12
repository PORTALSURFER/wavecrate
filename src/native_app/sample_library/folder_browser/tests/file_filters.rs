use super::*;

#[test]
fn name_filter_limits_selected_audio_files_and_clears_hidden_selection() {
    let root = temp_source_root("wavecrate-gui-name-filter");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("Deep Kick.wav");
    let snare = drums.join("Snare.wav");
    let hat = drums.join("Hat.wav");
    for file in [&kick, &snare, &hat] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&snare));

    browser.apply_message(FolderBrowserMessage::NameFilterInput(
        TextInputMessage::Changed {
            value: String::from("kick"),
        },
    ));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Deep Kick.wav"]
    );
    assert_eq!(browser.selected_file_id(), None);

    browser.apply_message(FolderBrowserMessage::NameFilterInput(
        TextInputMessage::Changed {
            value: String::new(),
        },
    ));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Deep Kick.wav", "Hat.wav", "Snare.wav"]
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn tag_filter_limits_selected_audio_files_and_clears_hidden_selection() {
    let root = temp_source_root("wavecrate-gui-tag-filter");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("Deep Kick.wav");
    let snare = drums.join("Snare.wav");
    let hat = drums.join("Hat.wav");
    for file in [&kick, &snare, &hat] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&snare));
    let tags_by_file = std::collections::HashMap::from([
        (
            path_id(&kick),
            vec![String::from("Drum"), String::from("Warm")],
        ),
        (path_id(&snare), vec![String::from("Drum")]),
        (path_id(&hat), vec![String::from("Metal")]),
    ]);

    browser.apply_message(FolderBrowserMessage::TagFilterInput(
        TextInputMessage::Changed {
            value: String::from("drum, warm"),
        },
    ));
    browser.retain_visible_file_selection_after_tag_filter(&tags_by_file);

    assert_eq!(
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Deep Kick.wav"]
    );
    assert_eq!(browser.selected_file_id(), None);

    browser.apply_message(FolderBrowserMessage::TagFilterInput(
        TextInputMessage::Changed {
            value: String::from("drum"),
        },
    ));
    browser.select_file(path_id(&kick));
    assert_eq!(
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Deep Kick.wav", "Snare.wav"]
    );
    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, &tags_by_file),
        Some(path_id(&snare))
    );
    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, &tags_by_file),
        None
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn tagged_file_count_matches_projected_filtered_samples() {
    let root = temp_source_root("wavecrate-gui-file-count-matching-tags");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("Deep Kick.wav");
    let snare = drums.join("Snare.wav");
    let hat = drums.join("Hat.wav");
    for file in [&kick, &snare, &hat] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    let tags_by_file = std::collections::HashMap::from([
        (
            path_id(&kick),
            vec![String::from("Drum"), String::from("Warm")],
        ),
        (path_id(&snare), vec![String::from("Drum")]),
        (path_id(&hat), vec![String::from("Metal")]),
    ]);

    browser.apply_message(FolderBrowserMessage::TagFilterInput(
        TextInputMessage::Changed {
            value: String::from("drum"),
        },
    ));

    assert_eq!(
        browser.selected_audio_file_count_matching_tags(&tags_by_file),
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .len()
    );
    assert_eq!(
        browser.selected_audio_file_count_matching_tags(&tags_by_file),
        2
    );
    let _ = fs::remove_dir_all(root);
}
