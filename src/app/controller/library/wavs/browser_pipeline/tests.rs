use super::base_stage::ensure_base_stage;
use super::folder_stage::ensure_folder_acceptance_stage;
use super::*;
use crate::app::controller::test_support::prepare_with_source_and_wav_entries;
use crate::app::state::{RootFolderFilterMode, SampleBrowserSort, VisibleRows};
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
    let root_mode = RootFolderFilterMode::RootOnly;
    let folder_hash = crate::app::controller::library::source_folders::folder_filter_fingerprint(
        Some(&selection),
        Some(&negated),
        root_mode,
    );

    ensure_base_stage(&mut controller);
    ensure_folder_acceptance_stage(
        &mut controller,
        Some(&selection),
        Some(&negated),
        root_mode,
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
