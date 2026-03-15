use super::stages::{BuildVisibleRowsParams, build_visible_rows_for_job};
use super::*;
use crate::app::controller::library::source_folders::FolderBrowserModel;
use crate::app::controller::test_support::prepare_with_source_and_wav_entries;
use crate::sample_sources::{Rating, WavEntry};
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn list_order_query_matches_sync_score_ranked_visible_rows() {
    let entries = vec![
        search_entry("zzabc.wav", Rating::NEUTRAL),
        search_entry("abc_extra.wav", Rating::NEUTRAL),
        search_entry("abc.wav", Rating::NEUTRAL),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());

    controller.set_browser_search("abc");

    let controller_visible = visible_indices(&controller);
    assert_ne!(controller_visible, vec![0, 1, 2]);

    let worker_scores = controller.ui_cache.browser.search.scores.clone();
    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let job = make_search_job(&source, "abc");
    let queue = SearchJobQueue::new();
    queue.send(make_search_job(&source, "abc"));
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;

    let worker_visible = build_visible_rows_for_job(
        &mut cache,
        BuildVisibleRowsParams {
            job: &job,
            has_query: true,
            scores: &worker_scores,
            entries_len: entries.len(),
            queue: &queue,
            generation,
            source_id: source.id.as_str(),
            has_folder_filters: false,
        },
    )
    .expect("expected worker visible rows");

    assert_eq!(worker_visible, controller_visible);
}

#[test]
fn folder_filter_visible_rows_match_sync_pipeline() {
    let entries = vec![
        search_entry("root.wav", Rating::NEUTRAL),
        search_entry("drums/kick.wav", Rating::NEUTRAL),
        search_entry("hits/snare.wav", Rating::NEUTRAL),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());
    controller.ui_cache.folders.models.insert(
        source.id.clone(),
        FolderBrowserModel {
            selected: BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]),
            negated: BTreeSet::from([PathBuf::from("hits")]),
            root_filter_mode: crate::app::state::RootFolderFilterMode::RootOnly,
            ..FolderBrowserModel::default()
        },
    );
    controller.rebuild_browser_lists();

    let controller_visible = visible_indices(&controller);
    assert_eq!(controller_visible, vec![0, 1]);

    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let job = SearchJob {
        folder_selection: Some(BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")])),
        folder_negated: Some(BTreeSet::from([PathBuf::from("hits")])),
        root_mode: crate::app::state::RootFolderFilterMode::RootOnly,
        ..make_search_job(&source, "")
    };
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        folder_selection: Some(BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")])),
        folder_negated: Some(BTreeSet::from([PathBuf::from("hits")])),
        root_mode: crate::app::state::RootFolderFilterMode::RootOnly,
        ..make_search_job(&source, "")
    });
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;
    let empty_scores: Arc<[Option<i64>]> = Arc::from([]);

    let worker_visible = build_visible_rows_for_job(
        &mut cache,
        BuildVisibleRowsParams {
            job: &job,
            has_query: false,
            scores: &empty_scores,
            entries_len: entries.len(),
            queue: &queue,
            generation,
            source_id: source.id.as_str(),
            has_folder_filters: true,
        },
    )
    .expect("expected worker visible rows");

    assert_eq!(worker_visible, controller_visible);
}

fn compact_entries(entries: &[WavEntry]) -> Vec<CompactSearchEntry> {
    entries
        .iter()
        .map(|entry| {
            let relative_path = entry.relative_path.to_string_lossy().to_string();
            let display_label = crate::app::view_model::sample_display_label(&entry.relative_path);
            CompactSearchEntry {
                display_label: display_label.into_boxed_str(),
                relative_path: relative_path.into_boxed_str(),
                tag: entry.tag,
                locked: entry.locked,
                last_played_at: entry.last_played_at,
            }
        })
        .collect()
}

fn search_entry(path: &str, tag: Rating) -> WavEntry {
    WavEntry {
        relative_path: PathBuf::from(path),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }
}

fn visible_indices(controller: &crate::app::controller::AppController) -> Vec<usize> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.visible_browser_index(row))
        .collect()
}

fn make_search_job(source: &crate::sample_sources::SampleSource, query: &str) -> SearchJob {
    SearchJob {
        request_id: 1,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        query: query.to_string(),
        filter: TriageFlagFilter::All,
        rating_filter: Default::default(),
        sort: SampleBrowserSort::ListOrder,
        similar_query: None,
        folder_selection: None,
        folder_negated: None,
        root_mode: crate::app::state::RootFolderFilterMode::AllDescendants,
    }
}
