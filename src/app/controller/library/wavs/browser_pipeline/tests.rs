use super::base_stage::ensure_base_stage;
use super::folder_stage::ensure_folder_acceptance_stage;
use super::*;
use crate::app::controller::test_support::prepare_with_source_and_wav_entries;
use crate::app::state::{
    BrowserDuplicateCleanupState, FolderFileScopeMode, PlaybackAgeFilterChip, SampleBrowserSort,
    SimilarQuery, TriageFlagFilter, VisibleRows,
};
use crate::sample_sources::Rating;
use std::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn base_stage_partitions_rows_by_triage_bucket() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("trash.wav", Rating::TRASH_1, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);

    assert_eq!(
        controller.ui_cache.browser.pipeline.base_rows,
        vec![0, 1, 2]
    );
    assert_eq!(controller.ui_cache.browser.pipeline.trash_rows, vec![1]);
    assert_eq!(controller.ui_cache.browser.pipeline.neutral_rows, vec![0]);
    assert_eq!(controller.ui_cache.browser.pipeline.keep_rows, vec![2]);
}

#[test]
fn base_stage_reuses_cached_fingerprint_without_rechecking_db_revision() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);
    let first_fingerprint = controller
        .ui_cache
        .browser
        .pipeline
        .base_fingerprint
        .clone();
    controller.cache.db.clear();
    let invalid_root = source.root.join("missing-after-cache");
    let selected = controller
        .library
        .sources
        .iter_mut()
        .find(|candidate| candidate.id == source.id)
        .expect("selected source");
    selected.root = invalid_root;

    ensure_base_stage(&mut controller);

    assert_eq!(
        controller.ui_cache.browser.pipeline.base_fingerprint,
        first_fingerprint
    );
    assert_eq!(controller.ui_cache.browser.pipeline.base_rows, vec![0, 1]);
}

#[test]
fn base_stage_rebuilds_after_same_path_tag_updates() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);

    ensure_base_stage(&mut controller);
    crate::app::controller::library::wavs::selection_ops::set_sample_tag_for_source(
        &mut controller,
        &source,
        Path::new("neutral.wav"),
        Rating::TRASH_1,
        false,
    )
    .expect("update tag");
    ensure_base_stage(&mut controller);

    assert_eq!(controller.ui_cache.browser.pipeline.trash_rows, vec![0]);
    assert_eq!(controller.ui_cache.browser.pipeline.keep_rows, vec![1]);
}

#[test]
fn folder_stage_acceptance_matches_root_and_negated_filters() {
    let entries = vec![
        search_entry("root.wav", Rating::NEUTRAL, None),
        search_entry("drums/kick.wav", Rating::NEUTRAL, None),
        search_entry("hits/snare.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    let selection = BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]);
    let negated = BTreeSet::from([PathBuf::from("hits")]);
    let file_scope_mode = FolderFileScopeMode::DirectOnly;
    let folder_hash = crate::app::controller::library::source_folders::folder_filter_fingerprint(
        Some(&selection),
        Some(&negated),
        file_scope_mode,
    );

    ensure_base_stage(&mut controller);
    ensure_folder_acceptance_stage(
        &mut controller,
        Some(&selection),
        Some(&negated),
        file_scope_mode,
        folder_hash,
        true,
    );

    assert_eq!(
        controller.ui_cache.browser.pipeline.folder_accepts_by_index,
        vec![true, true, false]
    );
}

#[test]
fn build_visible_rows_uses_all_fast_path_when_filters_are_idle() {
    let entries = vec![
        search_entry("a.wav", Rating::NEUTRAL, None),
        search_entry("b.wav", Rating::NEUTRAL, None),
        search_entry("c.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    let (visible, focused, loaded) = build_visible_rows(&mut controller, None, None);

    match visible {
        VisibleRows::All { total } => assert_eq!(total, 3),
        VisibleRows::List(rows) => panic!("expected fast-path all rows, got {:?}", rows),
    }
    assert_eq!(focused, None);
    assert_eq!(loaded, None);
}

#[test]
fn build_visible_rows_sort_stage_maps_focus_and_loaded_positions() {
    let entries = vec![
        search_entry("older.wav", Rating::NEUTRAL, Some(20)),
        search_entry("newer.wav", Rating::NEUTRAL, Some(10)),
        search_entry("never.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.sort = SampleBrowserSort::PlaybackAgeAsc;

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(1), Some(0));

    match visible {
        VisibleRows::All { total } => panic!("expected sorted list rows, got all {total}"),
        VisibleRows::List(rows) => assert_eq!(&*rows, &[2usize, 1usize, 0usize]),
    }
    assert_eq!(focused, Some(1));
    assert_eq!(loaded, Some(2));
}

#[test]
fn playback_age_filter_cache_stays_stable_until_the_next_week_boundary() {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

    let entries = vec![search_entry("aging.wav", Rating::NEUTRAL, Some(100))];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanWeek);

    let pre_boundary_now = 100 + WEEK_SECS - 2;
    let (visible_before, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        pre_boundary_now,
    );
    let fingerprint_before = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    let (visible_still_before, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        pre_boundary_now + 1,
    );
    let fingerprint_still_before = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    let (visible_after, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        pre_boundary_now + 2,
    );
    let fingerprint_after = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    match visible_before {
        VisibleRows::List(rows) => assert!(rows.is_empty()),
        VisibleRows::All { total } => {
            panic!("expected filtered list before boundary, got all {total}")
        }
    }
    match visible_still_before {
        VisibleRows::List(rows) => assert!(rows.is_empty()),
        VisibleRows::All { total } => {
            panic!("expected filtered list before boundary, got all {total}")
        }
    }
    match visible_after {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[0usize]),
        VisibleRows::All { total } => {
            panic!("expected filtered list after boundary, got all {total}")
        }
    }
    assert_eq!(fingerprint_before, fingerprint_still_before);
    assert_ne!(fingerprint_before, fingerprint_after);
}

#[test]
fn playback_age_filter_token_cache_tracks_base_snapshot_and_filter_shape() {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

    let entries = vec![
        search_entry("aging.wav", Rating::NEUTRAL, Some(100)),
        search_entry("fresh.wav", Rating::NEUTRAL, Some(100 + WEEK_SECS)),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanWeek);

    let now = 100 + WEEK_SECS - 5;
    let _ = super::visible_rows::build_visible_rows_with_now(&mut controller, None, None, now);
    let cached_before = controller
        .ui_cache
        .browser
        .pipeline
        .playback_age_token_cache;

    let _ = super::visible_rows::build_visible_rows_with_now(&mut controller, None, None, now + 1);
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .playback_age_token_cache,
        cached_before
    );

    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanMonth);
    let _ = super::visible_rows::build_visible_rows_with_now(&mut controller, None, None, now + 1);
    assert_ne!(
        controller
            .ui_cache
            .browser
            .pipeline
            .playback_age_token_cache,
        cached_before
    );
}

#[test]
fn targeted_playback_age_update_clears_cached_rollover_token() {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

    let entries = vec![search_entry("aging.wav", Rating::NEUTRAL, Some(100))];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanWeek);

    let now_unix_secs = 100 + WEEK_SECS - 2;
    let _ = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        now_unix_secs,
    );
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .playback_age_token_cache
            .as_ref()
            .and_then(|cache| cache.token),
        Some(100 + WEEK_SECS)
    );

    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .update_playback_age(0, Some(200))
    );
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .playback_age_token_cache
            .is_none()
    );

    let _ = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        now_unix_secs,
    );
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .playback_age_token_cache
            .as_ref()
            .and_then(|cache| cache.token),
        Some(200 + WEEK_SECS)
    );
}

#[test]
fn older_than_month_filter_ignores_the_week_rollover_cache_boundary() {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;
    const MONTH_SECS: i64 = 30 * 24 * 60 * 60;

    let entries = vec![search_entry("aging.wav", Rating::NEUTRAL, Some(200))];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanMonth);

    let just_before_week = 200 + WEEK_SECS - 1;
    let (visible_before_week, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        just_before_week,
    );
    let fingerprint_before_week = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    let (visible_after_week, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        just_before_week + 1,
    );
    let fingerprint_after_week = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    let (visible_after_month, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        200 + MONTH_SECS,
    );
    let fingerprint_after_month = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    match visible_before_week {
        VisibleRows::List(rows) => assert!(rows.is_empty()),
        VisibleRows::All { total } => {
            panic!("expected filtered list before month boundary, got all {total}")
        }
    }
    match visible_after_week {
        VisibleRows::List(rows) => assert!(rows.is_empty()),
        VisibleRows::All { total } => {
            panic!("expected filtered list after week boundary, got all {total}")
        }
    }
    match visible_after_month {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[0usize]),
        VisibleRows::All { total } => {
            panic!("expected filtered list after month boundary, got all {total}")
        }
    }
    assert_eq!(fingerprint_before_week, fingerprint_after_week);
    assert_ne!(fingerprint_before_week, fingerprint_after_month);
}

#[test]
fn duplicate_cleanup_projection_filters_missing_indices_and_remaps_focus() {
    let entries = vec![
        search_entry("keep.wav", Rating::KEEP_1, None),
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("trash.wav", Rating::TRASH_1, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.duplicate_cleanup = Some(BrowserDuplicateCleanupState::new(
        source.id,
        String::from("sample-1"),
        PathBuf::from("trash.wav"),
        String::from("trash.wav"),
        vec![2, 99, 0],
        vec![1.0, 0.8, 0.6],
        2,
    ));

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(0), Some(2));

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[2usize, 0usize]),
        VisibleRows::All { total } => panic!("expected duplicate-cleanup rows, got all {total}"),
    }
    assert_eq!(focused, Some(1));
    assert_eq!(loaded, Some(0));
}

#[test]
fn marked_only_filter_keeps_only_session_marked_rows() {
    let entries = vec![
        search_entry("one.wav", Rating::NEUTRAL, None),
        search_entry("two.wav", Rating::NEUTRAL, None),
        search_entry("three.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    controller.ui.browser.search.marked_only = true;

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(1), None);

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[1usize]),
        VisibleRows::All { total } => panic!("expected marked-only rows, got all {total}"),
    }
    assert_eq!(focused, Some(0));
    assert_eq!(loaded, None);
}

#[test]
fn mark_toggle_keeps_filtered_fingerprint_stable_when_marked_filter_is_off() {
    let entries = vec![
        search_entry("one.wav", Rating::NEUTRAL, None),
        search_entry("two.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    let _ = build_visible_rows(&mut controller, Some(0), None);
    let before = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();

    let _ = build_visible_rows(&mut controller, Some(0), None);
    let after = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    assert_eq!(before, after);
}

#[test]
fn folder_filter_build_does_not_fault_wav_pages_when_compact_cache_is_available() {
    let entries = vec![
        search_entry("root.wav", Rating::NEUTRAL, None),
        search_entry("drums/kick.wav", Rating::NEUTRAL, None),
        search_entry("hits/snare.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    controller.ui_cache.folders.models.insert(
        crate::app::controller::state::cache::FolderBrowserCacheKey {
            pane: crate::app::state::FolderPaneId::Upper,
            source_id: source.id,
        },
        crate::app::controller::library::source_folders::FolderBrowserModel {
            selected: BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]),
            negated: BTreeSet::from([PathBuf::from("hits")]),
            file_scope_mode: FolderFileScopeMode::DirectOnly,
            ..Default::default()
        },
    );
    clear_loaded_wav_pages(&mut controller);

    let (visible, _, _) = build_visible_rows(&mut controller, None, None);

    match visible {
        VisibleRows::List(rows) => {
            let mut visible_paths = rows
                .iter()
                .map(|index| {
                    controller.ui_cache.browser.pipeline.compact_entries[*index]
                        .relative_path
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .collect::<Vec<_>>();
            visible_paths.sort();
            assert_eq!(
                visible_paths,
                vec![String::from("drums/kick.wav"), String::from("root.wav")]
            );
        }
        VisibleRows::All { total } => panic!("expected folder-filtered rows, got all {total}"),
    }
    assert!(controller.wav_entries.pages.is_empty());
}

#[test]
fn playback_age_filter_build_does_not_fault_wav_pages_when_compact_cache_is_available() {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

    let entries = vec![
        search_entry("aging.wav", Rating::NEUTRAL, Some(100)),
        search_entry("fresh.wav", Rating::NEUTRAL, Some(100 + WEEK_SECS)),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanWeek);
    clear_loaded_wav_pages(&mut controller);

    let (visible, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        100 + WEEK_SECS + 1,
    );

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[0usize]),
        VisibleRows::All { total } => {
            panic!("expected playback-age-filtered rows, got all {total}")
        }
    }
    assert!(controller.wav_entries.pages.is_empty());
}

#[test]
fn text_query_branch_uses_search_scores_to_filter_visible_rows() {
    let entries = vec![
        search_entry("kick.wav", Rating::NEUTRAL, None),
        search_entry("snare.wav", Rating::NEUTRAL, None),
        search_entry("hat.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.search_query = String::from("snare");

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(1), Some(0));

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[1usize]),
        VisibleRows::All { total } => panic!("expected query-filtered rows, got all {total}"),
    }
    assert_eq!(focused, Some(0));
    assert_eq!(loaded, None);
}

#[test]
fn similar_query_branch_applies_similarity_order_after_filters() {
    let entries = vec![
        search_entry("keep_a.wav", Rating::KEEP_1, None),
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep_b.wav", Rating::KEEP_3, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.filter = TriageFlagFilter::Keep;
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similar_query = Some(SimilarQuery {
        sample_id: String::from("sample-keep"),
        label: String::from("keep"),
        indices: vec![0, 1, 2],
        scores: vec![0.4, 1.0, 0.9],
        anchor_index: None,
    });

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(2), Some(0));

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[2usize, 0usize]),
        VisibleRows::All { total } => panic!("expected similarity-sorted rows, got all {total}"),
    }
    assert_eq!(focused, Some(0));
    assert_eq!(loaded, Some(1));
}

fn search_entry(
    path: &str,
    tag: Rating,
    last_played_at: Option<i64>,
) -> crate::sample_sources::WavEntry {
    crate::sample_sources::WavEntry {
        relative_path: PathBuf::from(path),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag,
        looped: false,
        locked: false,
        missing: false,
        last_played_at,
    }
}

fn clear_loaded_wav_pages(controller: &mut crate::app::controller::AppController) {
    controller.wav_entries.pages.clear();
    controller.wav_entries.lookup.clear();
    controller.ui_cache.browser.pipeline.invalidate();
    controller.ui_cache.browser.search.invalidate();
}
