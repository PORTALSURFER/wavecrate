use super::*;
use crate::native_app::sample_library::folder_browser::commands::FilterFamily;
use crate::native_app::sample_library::folder_browser::model::{
    BROWSER_CURATION_SCOPES, BrowserCurationScope, HarvestFilter,
};
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery;
use std::path::Path;

fn visible_column_ids(browser: &FolderBrowserState) -> Vec<&str> {
    browser
        .visible_file_columns()
        .into_iter()
        .map(|column| column.id.as_str())
        .collect::<Vec<_>>()
}

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
fn playback_type_column_sorts_visible_rows_by_tagged_mode() {
    let root = temp_source_root("wavecrate-gui-playback-type-sort");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let alpha_shot = drums.join("alpha-shot.wav");
    let bravo_loop = drums.join("bravo-loop.wav");
    let charlie_shot = drums.join("charlie-shot.wav");
    for file in [&alpha_shot, &bravo_loop, &charlie_shot] {
        fs::write(file, []).expect("write sample");
    }
    let tags_by_file = std::collections::HashMap::from([
        (path_id(&alpha_shot), vec![String::from("one-shot")]),
        (path_id(&bravo_loop), vec![String::from("loop")]),
        (path_id(&charlie_shot), vec![String::from("oneshot")]),
    ]);
    let cached_sample_paths = Default::default();
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.apply_message(FolderBrowserMessage::SortFileColumn(String::from(
        "playback_type",
    )));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 0.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 3,
            viewport_start: 0,
            viewport_end: 3,
            window_start: 0,
            window_end: 3,
        },
    });

    assert_eq!(
        browser
            .visible_samples(VisibleSampleQuery {
                tags_by_file: &tags_by_file,
                cached_sample_paths: &cached_sample_paths,
            })
            .rows
            .iter()
            .map(|row| row.file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["bravo-loop.wav", "alpha-shot.wav", "charlie-shot.wav"]
    );

    browser.select_file(path_id(&bravo_loop));
    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, false, &tags_by_file),
        Some(path_id(&alpha_shot))
    );

    browser.select_file(path_id(&bravo_loop));
    browser.select_file_with_modifiers_matching_tags(
        path_id(&charlie_shot),
        PointerModifiers {
            shift: true,
            ..Default::default()
        },
        &tags_by_file,
    );
    assert_eq!(
        browser.selected_file_paths(),
        vec![alpha_shot.clone(), bravo_loop.clone(), charlie_shot.clone()]
    );
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
/// Normal browsing keeps workflow metadata out of the sample-list column set.
fn normal_browsing_hides_workflow_columns() {
    let browser = FolderBrowserState::load_default();

    assert_eq!(
        visible_column_ids(&browser),
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

#[test]
/// Selecting a concrete Harvest filter makes the Harvest column visible.
fn active_harvest_filter_shows_harvest_column() {
    let mut browser = FolderBrowserState::load_default();

    browser.set_harvest_filter(HarvestFilter::NeedsReview, true);

    assert_eq!(
        browser
            .visible_file_columns()
            .into_iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "harvest",
            "rating",
            "playback_type",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
}

#[test]
/// Enabling Curation makes the Curation column visible in its stored position.
fn active_curation_filter_shows_curation_column() {
    let mut browser = FolderBrowserState::load_default();

    browser.set_curation_scope(BrowserCurationScope::All, true);

    assert_eq!(
        visible_column_ids(&browser),
        vec![
            "name",
            "rating",
            "playback_type",
            "curation",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
}

#[test]
/// Every Curation dropdown mode participates in the same column visibility contract.
fn active_curation_scope_modes_show_curation_column() {
    for scope in BROWSER_CURATION_SCOPES {
        let mut browser = FolderBrowserState::load_default();

        browser.set_curation_scope(scope, true);

        assert!(
            visible_column_ids(&browser).contains(&"curation"),
            "{scope:?} should show the Curation column while active"
        );
    }
}

#[test]
/// Disabling Curation hides the column without discarding the selected scope.
fn inactive_curation_filter_hides_curation_column_and_preserves_scope() {
    let mut browser = FolderBrowserState::load_default();

    browser.set_curation_scope(BrowserCurationScope::Tags, true);
    browser.set_filter_family_enabled(FilterFamily::Curation, false);

    assert_eq!(browser.curation_scope(), BrowserCurationScope::Tags);
    assert!(!browser.curation_mode_enabled());
    assert!(!visible_column_ids(&browser).contains(&"curation"));

    browser.set_filter_family_enabled(FilterFamily::Curation, true);

    assert_eq!(browser.curation_scope(), BrowserCurationScope::Tags);
    assert!(browser.curation_mode_enabled());
    assert!(visible_column_ids(&browser).contains(&"curation"));
}

#[test]
/// The all-Harvest view still counts as an active Harvest workflow.
fn harvest_all_filter_counts_as_active_for_column_visibility() {
    let mut browser = FolderBrowserState::load_default();

    browser.set_harvest_filter(HarvestFilter::All, true);

    assert!(
        browser
            .visible_file_columns()
            .into_iter()
            .any(|column| column.id == "harvest")
    );
}

#[test]
fn collection_view_shows_source_folder_column() {
    let root = temp_source_root("wavecrate-gui-collection-folder-column");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let keep = drums.join("keep.wav");
    fs::write(&keep, []).expect("write sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");
    browser.set_file_collection_state(&keep, collection);

    assert_eq!(
        visible_column_ids(&browser),
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

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        visible_column_ids(&browser),
        vec![
            "name",
            "source_folder",
            "rating",
            "playback_type",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
/// Collection and Harvest workflows can surface their contextual columns independently.
fn collection_view_with_active_harvest_shows_contextual_columns() {
    let root = temp_source_root("wavecrate-gui-collection-harvest-columns");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let keep = drums.join("keep.wav");
    fs::write(&keep, []).expect("write sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");
    browser.set_file_collection_state(&keep, collection);

    browser.set_harvest_filter(HarvestFilter::NeedsReview, true);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        visible_column_ids(&browser),
        vec![
            "name",
            "harvest",
            "source_folder",
            "rating",
            "playback_type",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
/// Harvest, Collection, and Curation each contribute their dynamic columns when active.
fn collection_view_with_active_harvest_and_curation_shows_all_contextual_columns() {
    let root = temp_source_root("wavecrate-gui-collection-harvest-curation-columns");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let keep = drums.join("keep.wav");
    fs::write(&keep, []).expect("write sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");
    browser.set_file_collection_state(&keep, collection);

    browser.set_harvest_filter(HarvestFilter::NeedsReview, true);
    browser.set_curation_scope(BrowserCurationScope::All, true);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    assert_eq!(
        visible_column_ids(&browser),
        vec![
            "name",
            "harvest",
            "source_folder",
            "rating",
            "playback_type",
            "curation",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn collection_rows_show_source_relative_folder_path() {
    let root = temp_source_root("wavecrate-gui-collection-folder-cell");
    let kicks = root.join("drums").join("kicks");
    fs::create_dir_all(&kicks).expect("create kicks folder");
    let keep = kicks.join("keep.wav");
    fs::write(&keep, []).expect("write sample");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(0).expect("collection");
    browser.set_file_collection_state(&keep, collection);
    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

    let tags_by_file = Default::default();
    let cached_sample_paths = Default::default();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });
    let row = visible
        .rows
        .iter()
        .find(|row| row.file.id == path_id(&keep))
        .expect("collection row");

    assert_eq!(
        row.source_folder_path,
        Path::new("drums").join("kicks").to_string_lossy()
    );
    let _ = fs::remove_dir_all(root);
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
fn similarity_anchor_clears_when_selected_folder_changes() {
    let root = temp_source_root("wavecrate-gui-similarity-anchor-folder-change");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let anchor = drums.join("anchor.wav");
    fs::write(&anchor, []).expect("write anchor");
    fs::write(loops.join("loop.wav"), []).expect("write loop");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let drums_id = path_id(&drums);
    let loops_id = path_id(&loops);
    let anchor_id = path_id(&anchor);

    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        drums_id.clone(),
        Default::default(),
    ));
    browser.apply_message(FolderBrowserMessage::ToggleSimilarityAnchor(
        anchor_id.clone(),
    ));
    assert_eq!(browser.similarity_anchor_id(), Some(anchor_id.as_str()));

    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        drums_id,
        Default::default(),
    ));
    assert_eq!(
        browser.similarity_anchor_id(),
        Some(anchor_id.as_str()),
        "reselecting the same folder should not clear similarity mode"
    );

    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        loops_id,
        Default::default(),
    ));
    assert_eq!(browser.similarity_anchor_id(), None);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn similarity_anchor_clears_when_folder_keyboard_navigation_moves_focus() {
    let root = temp_source_root("wavecrate-gui-similarity-anchor-folder-keyboard");
    let drums = root.join("drums");
    let loops = root.join("loops");
    fs::create_dir_all(&drums).expect("create drums folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let anchor = drums.join("anchor.wav");
    fs::write(&anchor, []).expect("write anchor");
    fs::write(loops.join("loop.wav"), []).expect("write loop");
    let mut browser = FolderBrowserState::from_root(root.clone());
    let drums_id = path_id(&drums);
    let loops_id = path_id(&loops);
    let anchor_id = path_id(&anchor);

    browser.activate_folder(drums_id);
    browser.apply_message(FolderBrowserMessage::ToggleSimilarityAnchor(anchor_id));
    assert!(browser.navigate_selected_folder(1, false, false));

    assert_eq!(browser.selection.selected_folder, loops_id);
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
    assert_eq!(feedback.marker_x, 533.0);

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
fn sample_file_column_drag_feedback_tracks_resized_and_narrow_boundaries() {
    let mut browser = FolderBrowserState::load_default();

    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("name"),
        radiant::widgets::DragHandleMessage::started(Point::new(0.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("name"),
        radiant::widgets::DragHandleMessage::moved(Point::new(120.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("collection"),
        radiant::widgets::DragHandleMessage::started(Point::new(0.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::ResizeFileColumn(
        String::from("collection"),
        radiant::widgets::DragHandleMessage::moved(Point::new(-80.0, 0.0)),
    ));

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::started(Point::new(404.0, 0.0)),
    ));
    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::moved(Point::new(660.0, 0.0)),
    ));

    let feedback = browser
        .file_column_drag_feedback()
        .expect("resized column drag should project visual feedback");
    assert_eq!(feedback.marker_x, 643.0);

    browser.apply_message(FolderBrowserMessage::DragFileColumn(
        String::from("rating"),
        radiant::widgets::DragHandleMessage::ended(Point::new(660.0, 0.0)),
    ));

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
/// Hiding Harvest preserves the stored width and order for later reactivation.
fn hidden_harvest_column_preserves_resize_and_order_when_reactivated() {
    let mut browser = FolderBrowserState::load_default();

    browser.set_harvest_filter(HarvestFilter::NeedsReview, true);
    browser.resize_file_column(
        String::from("harvest"),
        DragHandleMessage::started(Point::new(0.0, 0.0)),
    );
    browser.resize_file_column(
        String::from("harvest"),
        DragHandleMessage::moved(Point::new(70.0, 0.0)),
    );
    browser.drag_file_column(
        String::from("harvest"),
        DragHandleMessage::started(Point::new(260.0, 0.0)),
    );
    browser.drag_file_column(
        String::from("harvest"),
        DragHandleMessage::ended(Point::new(560.0, 0.0)),
    );
    browser.set_harvest_filter(HarvestFilter::NeedsReview, false);

    assert!(
        browser
            .visible_file_columns()
            .into_iter()
            .all(|column| column.id != "harvest"),
        "inactive Harvest mode should hide the stored Harvest column"
    );

    browser.set_harvest_filter(HarvestFilter::NeedsReview, true);
    let visible = browser.visible_file_columns();

    assert_eq!(
        visible
            .iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "rating",
            "playback_type",
            "collection",
            "harvest",
            "extension",
            "size",
            "modified"
        ]
    );
    assert_eq!(
        visible
            .iter()
            .find(|column| column.id == "harvest")
            .map(|column| column.width),
        Some(144.0)
    );
}

#[test]
/// Hiding Curation preserves the stored width and position for later reactivation.
fn hidden_curation_column_preserves_resize_and_order_when_reactivated() {
    let mut browser = FolderBrowserState::load_default();

    browser.set_curation_scope(BrowserCurationScope::All, true);
    browser.resize_file_column(
        String::from("curation"),
        DragHandleMessage::started(Point::new(0.0, 0.0)),
    );
    browser.resize_file_column(
        String::from("curation"),
        DragHandleMessage::moved(Point::new(60.0, 0.0)),
    );
    browser.set_filter_family_enabled(FilterFamily::Curation, false);

    assert!(
        browser
            .visible_file_columns()
            .into_iter()
            .all(|column| column.id != "curation"),
        "inactive Curation mode should hide the stored Curation column"
    );

    browser.set_filter_family_enabled(FilterFamily::Curation, true);
    let visible = browser.visible_file_columns();

    assert_eq!(
        visible
            .iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "name",
            "rating",
            "playback_type",
            "curation",
            "collection",
            "extension",
            "size",
            "modified"
        ]
    );
    assert_eq!(
        visible
            .iter()
            .find(|column| column.id == "curation")
            .map(|column| column.width),
        Some(172.0)
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
