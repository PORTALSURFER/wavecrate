use super::base_stage::ensure_base_stage;
use super::folder_stage::ensure_folder_acceptance_stage;
use super::*;
use crate::app::controller::test_support::prepare_with_source_and_wav_entries;
use crate::app::state::{
    FolderFileScopeMode, PlaybackAgeFilterChip, SampleBrowserSort, VisibleRows,
};
use crate::sample_sources::Rating;
use std::collections::BTreeSet;
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
