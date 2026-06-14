use super::*;
use std::path::Path;
#[test]
fn sample_file_sort_toggles_by_column_and_navigation_uses_sorted_order() {
    let root = temp_source_root("wavecrate-gui-file-sort");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let small = drums.join("small.wav");
    let large = drums.join("large.wav");
    fs::write(&small, [0_u8; 8]).expect("write small");
    fs::write(&large, [0_u8; 128]).expect("write large");
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));

    browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from("size")));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["small.wav", "large.wav"]
    );

    browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from("size")));
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["large.wav", "small.wav"]
    );
    browser.select_file(path_id(&large));
    assert_eq!(browser.navigate_vertical(1, false), Some(path_id(&small)));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn history_column_label_reflects_last_played_behavior() {
    let browser = FolderBrowserState::load_default();

    let history = browser
        .visible_file_columns()
        .into_iter()
        .find(|column| column.id == "modified")
        .expect("history column");

    assert_eq!(history.label, "Last Played");
}

#[test]
fn history_column_uses_last_played_metadata_display_and_sort() {
    let root = temp_source_root("wavecrate-gui-last-played-column");
    let old = root.join("old.wav");
    let fresh = root.join("fresh.wav");
    let never = root.join("never.wav");
    for path in [&old, &fresh, &never] {
        fs::write(path, []).expect("write sample");
    }
    let db = SourceDatabase::open(&root).expect("open source db");
    db.upsert_file(Path::new("old.wav"), 0, 30)
        .expect("upsert old");
    db.set_last_played_at(Path::new("old.wav"), 1_000_000)
        .expect("set old last played");
    db.upsert_file(Path::new("fresh.wav"), 0, 10)
        .expect("upsert fresh");
    db.set_last_played_at(Path::new("fresh.wav"), 1_000_000_000)
        .expect("set fresh last played");
    db.upsert_file(Path::new("never.wav"), 0, 20)
        .expect("upsert never");
    let mut browser = FolderBrowserState::from_root(root.clone());

    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .find(|file| file.name == "never.wav")
            .map(|file| file.modified.as_str()),
        Some("Never")
    );

    browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from(
        "modified",
    )));

    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["fresh.wav", "old.wav", "never.wav"]
    );
    let _ = fs::remove_dir_all(root);
}
#[test]
fn similarity_anchor_toggle_sets_replaces_and_clears_active_anchor() {
    let root = temp_source_root("wavecrate-gui-similarity-anchor-toggle");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let kick = drums.join("kick.wav");
    let snare = drums.join("snare.wav");
    fs::write(&kick, []).expect("write kick");
    fs::write(&snare, []).expect("write snare");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let kick_id = path_id(&kick);
    let snare_id = path_id(&snare);

    browser.apply_message(FolderBrowserMessage::ToggleSimilarityAnchor(
        kick_id.clone(),
    ));
    assert_eq!(browser.similarity_anchor_id(), Some(kick_id.as_str()));
    assert!(browser.file_is_similarity_anchor(&kick_id));

    browser.apply_message(FolderBrowserMessage::ToggleSimilarityAnchor(
        snare_id.clone(),
    ));
    assert_eq!(browser.similarity_anchor_id(), Some(snare_id.as_str()));
    assert!(!browser.file_is_similarity_anchor(&kick_id));
    assert!(browser.file_is_similarity_anchor(&snare_id));

    browser.apply_message(FolderBrowserMessage::ToggleSimilarityAnchor(snare_id));
    assert_eq!(browser.similarity_anchor_id(), None);

    let _ = fs::remove_dir_all(root);
}
#[test]
fn similarity_mode_pins_anchor_and_sorts_scores_descending() {
    let root = temp_source_root("wavecrate-gui-similarity-sort");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let anchor = drums.join("anchor.wav");
    let near = drums.join("near.wav");
    let far = drums.join("far.wav");
    let missing = drums.join("missing.wav");
    for path in [&anchor, &near, &far, &missing] {
        fs::write(path, []).expect("write sample");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    let anchor_id = path_id(&anchor);
    let near_id = path_id(&near);
    let far_id = path_id(&far);
    let missing_id = path_id(&missing);

    browser.set_similarity_scores_for_tests(
        anchor_id.clone(),
        [(near_id.clone(), 0.82), (far_id.clone(), 0.31)]
            .into_iter()
            .collect(),
    );

    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            anchor_id.as_str(),
            near_id.as_str(),
            far_id.as_str(),
            missing_id.as_str()
        ]
    );
    assert_eq!(
        browser.similarity_display_strength_for_file(&anchor_id),
        Some(1.0)
    );
    assert!(
        browser
            .similarity_display_strength_for_file(&near_id)
            .is_some()
    );
    assert_eq!(
        browser.similarity_display_strength_for_file(&missing_id),
        None
    );

    let _ = fs::remove_dir_all(root);
}
#[test]
fn sample_file_column_resize_clamps_width() {
    let mut browser = FolderBrowserState::load_default();

    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("extension"),
        radiant::widgets::DragHandleMessage::started(Point::new(100.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("extension"),
        radiant::widgets::DragHandleMessage::moved(Point::new(-200.0, 0.0)),
    ));

    let extension_width = browser
        .visible_file_columns()
        .into_iter()
        .find(|column| column.id == "extension")
        .map(|column| column.width)
        .unwrap();
    assert_eq!(extension_width, MIN_FILE_COLUMN_WIDTH);
}
#[test]
fn sample_file_column_drag_reorders_columns() {
    let mut browser = FolderBrowserState::load_default();

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::started(Point::new(284.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::moved(Point::new(560.0, 0.0)),
    ));
    assert_eq!(
        browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "rating",
            "playback_type",
            "collection",
            "extension",
            "size",
            "modified"
        ],
        "column order should stay stable until the drag is released"
    );
    let feedback = browser
        .file_column_drag_feedback()
        .expect("active column drag should project visual feedback");
    assert_eq!(feedback.label, "Rating");
    assert_eq!(feedback.pointer, Point::new(560.0, 0.0));
    assert_eq!(feedback.marker_x, 534.0);

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::ended(Point::new(560.0, 0.0)),
    ));

    assert_eq!(browser.file_column_drag_feedback(), None);
    assert_eq!(
        browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "playback_type",
            "collection",
            "extension",
            "rating",
            "size",
            "modified"
        ]
    );
}
#[test]
fn sample_file_column_drag_cancel_clears_feedback_without_reorder() {
    let mut browser = FolderBrowserState::load_default();

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::started(Point::new(284.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::moved(Point::new(560.0, 0.0)),
    ));
    assert!(browser.file_column_drag_feedback().is_some());

    browser.apply_message(FolderBrowserMessage::CancelFileColumnDrag);

    assert_eq!(browser.file_column_drag_feedback(), None);
    assert_eq!(
        browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "rating",
            "playback_type",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
}
