use super::support::*;

#[test]
fn playback_age_filter_visible_rows_match_sync_pipeline() {
    let now_unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let never = search_entry("never.wav", Rating::NEUTRAL);
    let mut month = search_entry("month.wav", Rating::NEUTRAL);
    let mut week = search_entry("week.wav", Rating::NEUTRAL);
    let mut fresh = search_entry("fresh.wav", Rating::NEUTRAL);
    month.last_played_at = Some(now_unix_secs.saturating_sub(40 * 24 * 60 * 60));
    week.last_played_at = Some(now_unix_secs.saturating_sub(10 * 24 * 60 * 60));
    fresh.last_played_at = Some(now_unix_secs.saturating_sub(2 * 24 * 60 * 60));
    let entries = vec![never, month, week, fresh];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());

    controller.set_browser_playback_age_filter(
        crate::app::state::PlaybackAgeFilterChip::NeverPlayed,
        false,
    );
    controller.set_browser_playback_age_filter(
        crate::app::state::PlaybackAgeFilterChip::OlderThanMonth,
        true,
    );

    let controller_visible = visible_indices(&controller);
    assert_eq!(controller_visible, vec![0, 1]);

    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let job = SearchJob {
        playback_age_filter: BTreeSet::from([
            crate::app::state::PlaybackAgeFilterChip::NeverPlayed,
            crate::app::state::PlaybackAgeFilterChip::OlderThanMonth,
        ]),
        playback_age_now_unix_secs: now_unix_secs,
        ..make_search_job(&source, "")
    };
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        playback_age_filter: BTreeSet::from([
            crate::app::state::PlaybackAgeFilterChip::NeverPlayed,
            crate::app::state::PlaybackAgeFilterChip::OlderThanMonth,
        ]),
        playback_age_now_unix_secs: now_unix_secs,
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
            has_folder_filters: false,
        },
    )
    .expect("expected worker visible rows");

    assert_eq!(worker_visible, controller_visible);
}

#[test]
fn playback_age_sort_visible_rows_match_sync_pipeline() {
    let now_unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let never = search_entry("never.wav", Rating::NEUTRAL);
    let mut month = search_entry("month.wav", Rating::NEUTRAL);
    let mut week = search_entry("week.wav", Rating::NEUTRAL);
    let mut fresh = search_entry("fresh.wav", Rating::NEUTRAL);
    month.last_played_at = Some(now_unix_secs.saturating_sub(40 * 24 * 60 * 60));
    week.last_played_at = Some(now_unix_secs.saturating_sub(10 * 24 * 60 * 60));
    fresh.last_played_at = Some(now_unix_secs.saturating_sub(2 * 24 * 60 * 60));
    let entries = vec![never, month, week, fresh];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());

    controller.set_browser_sort(SampleBrowserSort::PlaybackAgeAsc);
    let controller_visible = visible_indices(&controller);
    assert_eq!(controller_visible, vec![0, 1, 2, 3]);

    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let job = SearchJob {
        sort: SampleBrowserSort::PlaybackAgeAsc,
        playback_age_now_unix_secs: now_unix_secs,
        ..make_search_job(&source, "")
    };
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        sort: SampleBrowserSort::PlaybackAgeAsc,
        playback_age_now_unix_secs: now_unix_secs,
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
            has_folder_filters: false,
        },
    )
    .expect("expected worker visible rows");

    assert_eq!(worker_visible, controller_visible);
}
