use super::*;
use crate::native_app::sample_library::folder_browser::model::{
    BrowserCurationScope, PlaybackTypeFilter,
};
use crate::native_app::sample_library::folder_browser::projection::VisibleSampleQuery;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn tagged_file_window_materializes_requested_range_without_holes() {
    let root = temp_source_root("wavecrate-gui-tag-filter-window");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let files = (0..90)
        .map(|index| drums.join(format!("sample_{index:03}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, []).expect("write sample file");
    }
    let tags_by_file = files
        .iter()
        .enumerate()
        .filter(|(index, _file)| index % 2 == 0)
        .map(|(_index, file)| (path_id(file), vec![String::from("Drum")]))
        .collect::<std::collections::HashMap<_, _>>();
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.apply_message(FolderBrowserMessage::TagFilterInput(
        TextInputMessage::Changed {
            value: String::from("drum"),
        },
    ));

    let window_files = browser.selected_audio_file_window_matching_tags(
        radiant::prelude::VirtualListWindow {
            total_items: 45,
            viewport_start: 30,
            viewport_end: 40,
            window_start: 28,
            window_end: 42,
        },
        &tags_by_file,
    );

    assert_eq!(window_files.total_count, 45);
    assert_eq!(window_files.rows.len(), 14);
    assert_eq!(window_files.rows[0].name, "sample_056.wav");
    assert_eq!(window_files.rows[13].name, "sample_082.wav");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn curation_mode_filters_recent_and_locked_keep_rows() {
    let root = temp_source_root("wavecrate-gui-curation-filter");
    let empty = root.join("empty.wav");
    let tagged = root.join("tagged.wav");
    let recent = root.join("recent.wav");
    let locked = root.join("locked.wav");
    for file in [&empty, &tagged, &recent, &locked] {
        fs::write(file, []).expect("write sample file");
    }
    let now = curation_test_now();
    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.set_file_rating_state(&recent, Rating::KEEP_1, false));
    assert!(browser.set_file_last_curated_at(&recent, now - 60));
    assert!(browser.set_file_rating_state(&locked, Rating::KEEP_3, true));
    assert!(browser.set_file_last_curated_at(&locked, now - 60 * 60 * 24 * 90));
    browser.apply_message(FolderBrowserMessage::SetCurationScope(
        BrowserCurationScope::All,
        true,
    ));
    let tags_by_file = HashMap::from([
        (
            path_id(&tagged),
            vec![String::from("kick"), String::from("one-shot")],
        ),
        (
            path_id(&recent),
            vec![String::from("kick"), String::from("one-shot")],
        ),
        (
            path_id(&locked),
            vec![String::from("kick"), String::from("one-shot")],
        ),
    ]);

    let window_files = browser.selected_audio_file_window_matching_tags(
        radiant::prelude::VirtualListWindow {
            total_items: 4,
            viewport_start: 0,
            viewport_end: 4,
            window_start: 0,
            window_end: 4,
        },
        &tags_by_file,
    );

    assert_eq!(window_files.total_count, 2);
    assert_eq!(
        window_files
            .rows
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["empty.wav", "tagged.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn curation_focus_override_reveals_selected_recent_row_temporarily() {
    let root = temp_source_root("wavecrate-gui-curation-focus-override");
    let empty = root.join("empty.wav");
    let recent = root.join("recent.wav");
    for file in [&empty, &recent] {
        fs::write(file, []).expect("write sample file");
    }
    let now = curation_test_now();
    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.set_file_last_curated_at(&recent, now - 60));
    browser.apply_message(FolderBrowserMessage::SetCurationScope(
        BrowserCurationScope::All,
        true,
    ));
    browser.select_file(path_id(&recent));
    let tags_by_file = HashMap::new();

    assert_eq!(
        browser.selected_audio_file_index_matching_tags(&tags_by_file),
        None,
        "recent selected rows should normally be hidden by curation"
    );
    assert!(browser.reveal_selected_curation_focus_if_hidden(&tags_by_file));
    assert!(
        browser
            .selected_audio_file_index_matching_tags(&tags_by_file)
            .is_some(),
        "history reveal should make the selected hidden curation row visible"
    );
    assert_eq!(
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["empty.wav", "recent.wav"]
    );

    assert!(browser.clear_curation_focus_override());
    assert_eq!(
        browser.selected_audio_file_index_matching_tags(&tags_by_file),
        None
    );
    assert_eq!(
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["empty.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn curation_scope_modes_filter_rating_and_tag_work_separately() {
    let root = temp_source_root("wavecrate-gui-curation-scope-filter");
    let complete_unrated = root.join("complete-unrated.wav");
    let complete_low_rating = root.join("complete-low-rating.wav");
    let missing_tags_rated = root.join("missing-tags-rated.wav");
    let complete_rated = root.join("complete-rated.wav");
    for file in [
        &complete_unrated,
        &complete_low_rating,
        &missing_tags_rated,
        &complete_rated,
    ] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    assert!(browser.set_file_rating_state(&complete_low_rating, Rating::KEEP_1, false));
    assert!(browser.set_file_rating_state(&missing_tags_rated, Rating::KEEP_3, false));
    assert!(browser.set_file_rating_state(&complete_rated, Rating::KEEP_3, false));
    let stale_curated_at = curation_test_now() - 60 * 60 * 24 * 90;
    assert!(browser.set_file_last_curated_at(&complete_low_rating, stale_curated_at));
    assert!(browser.set_file_last_curated_at(&missing_tags_rated, stale_curated_at));
    let complete_tags = vec![String::from("kick"), String::from("one-shot")];
    let tags_by_file = HashMap::from([
        (path_id(&complete_unrated), complete_tags.clone()),
        (path_id(&complete_low_rating), complete_tags.clone()),
        (path_id(&complete_rated), complete_tags),
    ]);

    browser.apply_message(FolderBrowserMessage::SetCurationScope(
        BrowserCurationScope::Ratings,
        true,
    ));
    let rating_rows = browser.selected_audio_file_window_matching_tags(
        radiant::prelude::VirtualListWindow {
            total_items: 4,
            viewport_start: 0,
            viewport_end: 4,
            window_start: 0,
            window_end: 4,
        },
        &tags_by_file,
    );
    assert_eq!(
        rating_rows
            .rows
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["complete-unrated.wav", "complete-low-rating.wav"]
    );

    browser.apply_message(FolderBrowserMessage::SetCurationScope(
        BrowserCurationScope::Tags,
        true,
    ));
    let tag_rows = browser.selected_audio_file_window_matching_tags(
        radiant::prelude::VirtualListWindow {
            total_items: 4,
            viewport_start: 0,
            viewport_end: 4,
            window_start: 0,
            window_end: 4,
        },
        &tags_by_file,
    );
    assert_eq!(
        tag_rows
            .rows
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["missing-tags-rated.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn curation_random_navigation_draws_from_current_priority_bucket() {
    let root = temp_source_root("wavecrate-gui-curation-random");
    let empty_a = root.join("empty-a.wav");
    let empty_b = root.join("empty-b.wav");
    let tagged = root.join("tagged.wav");
    for file in [&empty_a, &empty_b, &tagged] {
        fs::write(file, []).expect("write sample file");
    }
    let tags_by_file = HashMap::from([(
        path_id(&tagged),
        vec![String::from("kick"), String::from("one-shot")],
    )]);
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.apply_message(FolderBrowserMessage::SetCurationScope(
        BrowserCurationScope::All,
        true,
    ));
    browser.select_file(path_id(&empty_a));
    browser.toggle_random_navigation();

    for _ in 0..6 {
        let selected = browser
            .navigate_vertical_matching_tags(1, false, false, &tags_by_file)
            .expect("random curation navigation target");
        assert_ne!(selected, path_id(&tagged));
    }
    let _ = fs::remove_dir_all(root);
}

fn curation_test_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64
}

#[test]
fn visible_samples_clamps_stale_scrollbar_window_without_blank_rows() {
    let root = temp_source_root("wavecrate-gui-stale-scrollbar-window");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let files = (0..24)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 9_990.0 * 22.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 10_000,
            viewport_start: 9_990,
            viewport_end: 10_000,
            window_start: 9_986,
            window_end: 10_000,
        },
    });

    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert_eq!(visible.total_count, 24);
    assert_eq!(visible.window.viewport_start, 14);
    assert_eq!(visible.window.window_start, 10);
    assert_eq!(visible.rows.len(), visible.window.window_len());
    assert_eq!(
        visible.rows.first().map(|row| row.file.name.as_str()),
        Some("sample_10.wav")
    );
    assert_eq!(
        visible.rows.last().map(|row| row.file.name.as_str()),
        Some("sample_23.wav")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn visible_samples_repairs_stale_projection_cache_without_blank_rows() {
    let root = temp_source_root("wavecrate-gui-stale-row-projection");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let files = (0..8)
        .map(|index| drums.join(format!("sample_{index:02}.wav")))
        .collect::<Vec<_>>();
    for file in &files {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    let drums_id = path_id(&drums);
    browser.activate_folder(drums_id.clone());
    let _ = browser.selected_audio_files();

    let folder = browser
        .tree
        .folders
        .first_mut()
        .and_then(|root| root.find_mut(&drums_id))
        .expect("selected test folder should exist");
    folder.files.remove(3);
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 0.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 8,
            viewport_start: 0,
            viewport_end: 8,
            window_start: 0,
            window_end: 8,
        },
    });

    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert_eq!(visible.total_count, 7);
    assert_eq!(visible.window.total_items, 7);
    assert_eq!(visible.rows.len(), visible.window.window_len());
    assert_eq!(
        visible
            .rows
            .iter()
            .map(|row| row.file.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "sample_00.wav",
            "sample_01.wav",
            "sample_02.wav",
            "sample_04.wav",
            "sample_05.wav",
            "sample_06.wav",
            "sample_07.wav",
        ]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn copied_file_flash_projects_to_visible_rows_and_clears() {
    let root = temp_source_root("wavecrate-gui-copy-flash-visible-row");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let hat = drums.join("hat.wav");
    let kick = drums.join("kick.wav");
    for file in [&hat, &kick] {
        fs::write(file, [0_u8; 8]).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 0.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: 2,
            viewport_start: 0,
            viewport_end: 2,
            window_start: 0,
            window_end: 2,
        },
    });

    browser.flash_copied_file_paths([hat.clone()]);
    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert!(
        visible
            .rows
            .iter()
            .any(|row| row.file.id == path_id(&hat) && row.copy_flash)
    );
    assert!(
        visible
            .rows
            .iter()
            .any(|row| row.file.id == path_id(&kick) && !row.copy_flash)
    );

    let mut frames = 0;
    while browser.copy_flash_active() {
        frames += 1;
        assert!(
            frames <= 12,
            "copy flash should clear after its frame budget"
        );
        browser.advance_copy_flash_frame();
    }
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert!(visible.rows.iter().all(|row| !row.copy_flash));
    let _ = fs::remove_dir_all(root);
}

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
fn rating_filter_limits_visible_samples_and_can_combine_levels() {
    let root = temp_source_root("wavecrate-gui-rating-filter");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let favorite = drums.join("favorite.wav");
    let maybe = drums.join("maybe.wav");
    let rejected = drums.join("rejected.wav");
    let unrated = drums.join("unrated.wav");
    for file in [&favorite, &maybe, &rejected, &unrated] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    assert!(browser.set_file_rating_state(&favorite, Rating::KEEP_3, true));
    assert!(browser.set_file_rating_state(&maybe, Rating::KEEP_1, false));
    assert!(browser.set_file_rating_state(&rejected, Rating::TRASH_3, false));
    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();

    browser.apply_message(FolderBrowserMessage::ToggleRatingFilter(-3, true));
    browser.apply_message(FolderBrowserMessage::ToggleRatingFilter(4, true));

    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert_eq!(visible.total_count, 2);
    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["favorite.wav", "rejected.wav"]
    );

    browser.apply_message(FolderBrowserMessage::ToggleRatingFilter(-3, false));

    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["favorite.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rating_filter_can_show_unrated_samples() {
    let root = temp_source_root("wavecrate-gui-unrated-filter");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let keep = drums.join("keep.wav");
    let unrated = drums.join("unrated.wav");
    for file in [&keep, &unrated] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    assert!(browser.set_file_rating_state(&keep, Rating::KEEP_1, false));

    browser.apply_message(FolderBrowserMessage::ToggleRatingFilter(0, true));

    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["unrated.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rating_filter_clears_selection_hidden_by_filter() {
    let root = temp_source_root("wavecrate-gui-rating-filter-selection");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let keep = drums.join("keep.wav");
    let neutral = drums.join("neutral.wav");
    for file in [&keep, &neutral] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    assert!(browser.set_file_rating_state(&keep, Rating::KEEP_1, false));
    browser.select_file(path_id(&neutral));

    browser.apply_message(FolderBrowserMessage::ToggleRatingFilter(1, true));

    assert_eq!(browser.selected_file_id(), None);
    assert_eq!(
        browser
            .selected_audio_files()
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["keep.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playback_type_filter_limits_visible_samples_and_combines_modes() {
    let root = temp_source_root("wavecrate-gui-playback-type-filter");
    let drums = root.join("drums");
    fs::create_dir_all(&drums).expect("create drums folder");
    let loop_file = drums.join("loop.wav");
    let shot = drums.join("shot.wav");
    let unknown = drums.join("unknown.wav");
    for file in [&loop_file, &shot, &unknown] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.activate_folder(path_id(&drums));
    browser.select_file(path_id(&unknown));
    let tags_by_file = HashMap::from([
        (path_id(&loop_file), vec![String::from("loop")]),
        (path_id(&shot), vec![String::from("one-shot")]),
    ]);
    let cached_sample_paths = HashSet::new();

    browser.apply_message(FolderBrowserMessage::TogglePlaybackTypeFilter(
        PlaybackTypeFilter::Loop,
        true,
    ));
    browser.retain_visible_file_selection_after_tag_filter(&tags_by_file);

    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });
    assert_eq!(visible.total_count, 1);
    assert_eq!(
        visible
            .rows
            .iter()
            .map(|row| row.file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["loop.wav"]
    );
    assert_eq!(browser.selected_file_id(), None);

    browser.apply_message(FolderBrowserMessage::TogglePlaybackTypeFilter(
        PlaybackTypeFilter::OneShot,
        true,
    ));

    assert_eq!(
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["loop.wav", "shot.wav"]
    );

    browser.apply_message(FolderBrowserMessage::TogglePlaybackTypeFilter(
        PlaybackTypeFilter::Loop,
        false,
    ));

    assert_eq!(
        browser.selected_audio_file_count_matching_tags(&tags_by_file),
        1
    );
    assert_eq!(
        browser
            .selected_audio_files_matching_tags(&tags_by_file)
            .into_iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["shot.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn playback_type_filter_applies_to_subtree_and_collection_windows() {
    let root = temp_source_root("wavecrate-gui-playback-type-filter-scopes");
    let drums = root.join("drums");
    let loops = drums.join("loops");
    let shots = drums.join("shots");
    fs::create_dir_all(&loops).expect("create loops folder");
    fs::create_dir_all(&shots).expect("create shots folder");
    let loop_file = loops.join("loop.wav");
    let shot = shots.join("shot.wav");
    for file in [&loop_file, &shot] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    let collection = SampleCollection::new(1).expect("collection");
    browser.set_file_collection_state(&loop_file, collection);
    browser.set_file_collection_state(&shot, collection);
    let tags_by_file = HashMap::from([
        (path_id(&loop_file), vec![String::from("loop")]),
        (path_id(&shot), vec![String::from("one-shot")]),
    ]);
    let cached_sample_paths = HashSet::new();

    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        path_id(&drums),
        Default::default(),
    ));
    browser.apply_message(FolderBrowserMessage::ToggleFolderSubtreeListing);
    browser.apply_message(FolderBrowserMessage::TogglePlaybackTypeFilter(
        PlaybackTypeFilter::Loop,
        true,
    ));

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
        vec!["loop.wav"]
    );

    browser.apply_message(FolderBrowserMessage::ActivateCollection(collection));

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
        vec!["loop.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_subtree_listing_includes_descendant_samples_when_enabled() {
    let root = temp_source_root("wavecrate-gui-subtree-listing");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    let loops = root.join("loops");
    fs::create_dir_all(&kicks).expect("create nested kicks folder");
    fs::create_dir_all(&loops).expect("create loops folder");
    let kick = kicks.join("kick.wav");
    let loop_file = loops.join("loop.wav");
    let snare = drums.join("snare.wav");
    for file in [&kick, &loop_file, &snare] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        path_id(&drums),
        Default::default(),
    ));
    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();

    assert!(!browser.folder_subtree_listing_enabled());
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["snare.wav"]
    );

    browser.apply_message(FolderBrowserMessage::ToggleFolderSubtreeListing);

    assert!(browser.folder_subtree_listing_enabled());
    assert!(
        browser
            .selected_folder_status_label()
            .contains("2 audio incl subfolders")
    );
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "snare.wav"]
    );
    assert_eq!(
        browser
            .visible_samples(VisibleSampleQuery {
                tags_by_file: &tags_by_file,
                cached_sample_paths: &cached_sample_paths,
            })
            .total_count,
        2
    );

    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        path_id(&root),
        Default::default(),
    ));

    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["kick.wav", "loop.wav", "snare.wav"]
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn folder_subtree_listing_materializes_deep_scroll_window() {
    let root = temp_source_root("wavecrate-gui-subtree-listing-window");
    let mut files = Vec::new();
    for index in 0..64 {
        let folder = root.join(format!("group_{:02}", index / 8));
        fs::create_dir_all(&folder).expect("create grouped folder");
        let file = folder.join(format!("sample_{index:03}.wav"));
        fs::write(&file, []).expect("write sample file");
        files.push(file);
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        path_id(&root),
        Default::default(),
    ));
    browser.apply_message(FolderBrowserMessage::ToggleFolderSubtreeListing);
    browser.apply_file_view_window_change(radiant::prelude::VirtualListWindowChange {
        offset_y: 48.0 * 22.0,
        row_height: 22.0,
        window: radiant::prelude::VirtualListWindow {
            total_items: files.len(),
            viewport_start: 48,
            viewport_end: 54,
            window_start: 46,
            window_end: 56,
        },
    });

    let tags_by_file = HashMap::new();
    let cached_sample_paths = HashSet::new();
    let visible = browser.visible_samples(VisibleSampleQuery {
        tags_by_file: &tags_by_file,
        cached_sample_paths: &cached_sample_paths,
    });

    assert_eq!(visible.total_count, files.len());
    assert_eq!(visible.window.window_start, 46);
    assert_eq!(
        visible
            .rows
            .iter()
            .map(|row| row.file.name.as_str())
            .collect::<Vec<_>>(),
        (46..56)
            .map(|index| format!("sample_{index:03}.wav"))
            .collect::<Vec<_>>()
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn disabling_folder_subtree_listing_drops_hidden_nested_file_selection() {
    let root = temp_source_root("wavecrate-gui-subtree-listing-selection");
    let drums = root.join("drums");
    let kicks = drums.join("kicks");
    fs::create_dir_all(&kicks).expect("create nested kicks folder");
    let kick = kicks.join("kick.wav");
    let snare = drums.join("snare.wav");
    for file in [&kick, &snare] {
        fs::write(file, []).expect("write sample file");
    }
    let mut browser = FolderBrowserState::from_root(root.clone());
    browser.apply_message(FolderBrowserMessage::ActivateFolder(
        path_id(&drums),
        Default::default(),
    ));
    browser.apply_message(FolderBrowserMessage::ToggleFolderSubtreeListing);
    browser.select_file(path_id(&kick));

    assert_eq!(browser.selected_file_id(), Some(path_id(&kick).as_str()));

    browser.apply_message(FolderBrowserMessage::ToggleFolderSubtreeListing);

    assert!(!browser.folder_subtree_listing_enabled());
    assert_eq!(browser.selected_file_id(), None);
    assert_eq!(
        browser
            .selected_audio_files()
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        vec!["snare.wav"]
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
        browser.navigate_vertical_matching_tags(1, false, false, &tags_by_file),
        Some(path_id(&snare))
    );
    assert_eq!(
        browser.navigate_vertical_matching_tags(1, false, false, &tags_by_file),
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
