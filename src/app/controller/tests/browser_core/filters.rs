use super::*;

#[test]
fn playback_age_bucket_classifies_expected_ranges() {
    let now_unix_secs = 90_i64 * 24 * 60 * 60;

    assert_eq!(
        crate::app::state::PlaybackAgeBucket::from_last_played_at(None, now_unix_secs),
        crate::app::state::PlaybackAgeBucket::NeverPlayed
    );
    assert_eq!(
        crate::app::state::PlaybackAgeBucket::from_last_played_at(
            Some(now_unix_secs - (31 * 24 * 60 * 60)),
            now_unix_secs,
        ),
        crate::app::state::PlaybackAgeBucket::OlderThanMonth
    );
    assert_eq!(
        crate::app::state::PlaybackAgeBucket::from_last_played_at(
            Some(now_unix_secs - (8 * 24 * 60 * 60)),
            now_unix_secs,
        ),
        crate::app::state::PlaybackAgeBucket::OlderThanWeek
    );
    assert_eq!(
        crate::app::state::PlaybackAgeBucket::from_last_played_at(
            Some(now_unix_secs - (6 * 24 * 60 * 60)),
            now_unix_secs,
        ),
        crate::app::state::PlaybackAgeBucket::Fresh
    );
    assert_eq!(
        crate::app::state::PlaybackAgeBucket::from_last_played_at(
            Some(now_unix_secs + (2 * 24 * 60 * 60)),
            now_unix_secs,
        ),
        crate::app::state::PlaybackAgeBucket::Fresh
    );
}

#[test]
fn sample_browser_indices_track_tags() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash.wav", crate::sample_sources::Rating::TRASH_3),
        sample_entry("neutral.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("keep.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("neutral.wav"));
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("keep.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert_eq!(controller.browser_indices(TriageFlagColumn::Trash).len(), 1);
    assert_eq!(
        controller.browser_indices(TriageFlagColumn::Neutral).len(),
        1
    );
    assert_eq!(controller.browser_indices(TriageFlagColumn::Keep).len(), 1);
    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);

    let selected = controller.ui.browser.selection.selected.unwrap();
    assert_eq!(selected.column, TriageFlagColumn::Neutral);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    let loaded = controller.ui.browser.selection.loaded.unwrap();
    assert_eq!(loaded.column, TriageFlagColumn::Keep);
    assert_eq!(controller.ui.browser.selection.loaded_visible, Some(2));
}

#[test]
fn browser_filter_limits_visible_rows() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash.wav", crate::sample_sources::Rating::TRASH_3),
        sample_entry("neutral.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("keep.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_filter(TriageFlagFilter::Keep);
    assert_eq!(visible_indices(&controller), vec![2]);
    controller.set_browser_filter(TriageFlagFilter::Trash);
    assert_eq!(visible_indices(&controller), vec![0]);
    controller.set_browser_filter(TriageFlagFilter::Untagged);
    assert_eq!(visible_indices(&controller), vec![1]);
    controller.set_browser_filter(TriageFlagFilter::All);
    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
}

#[test]
fn browser_rating_filter_limits_visible_rows() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);

    controller.set_browser_rating_filter(-2, false);
    assert_eq!(visible_indices(&controller), vec![1]);
    let rating_filter_revision = controller.ui.projection_revisions.browser_search;
    assert!(controller.refresh_projection_revision_bus());
    assert_ne!(
        controller.ui.projection_revisions.browser_search,
        rating_filter_revision
    );

    controller.set_browser_rating_filter(2, true);
    assert_eq!(visible_indices(&controller), vec![1, 5]);

    controller.set_browser_rating_filter(3, false);
    assert_eq!(visible_indices(&controller), vec![6]);

    controller.set_browser_rating_filter(4, false);
    assert_eq!(visible_indices(&controller), vec![7]);

    controller.clear_browser_rating_filter();
    assert_eq!(visible_indices(&controller), vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn invert_browser_rating_filter_selects_every_level_except_clicked_keep_chip() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);

    controller.invert_browser_rating_filter(4);

    assert_eq!(
        controller
            .ui
            .browser
            .search
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 0, 1, 2, 3]
    );
    assert_eq!(visible_indices(&controller), vec![0, 1, 2, 3, 4, 5, 6]);
}

#[test]
fn invert_browser_rating_filter_clears_when_same_exclusion_is_reclicked() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);

    controller.invert_browser_rating_filter(4);
    assert_eq!(
        controller
            .ui
            .browser
            .search
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 0, 1, 2, 3]
    );

    controller.invert_browser_rating_filter(4);

    assert!(controller.ui.browser.search.rating_filter.is_empty());
    assert_eq!(visible_indices(&controller), vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn locked_keep_filter_only_matches_locked_keep_rows() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    let mut locked_trash = sample_entry("locked_trash.wav", Rating::TRASH_3);
    locked_trash.locked = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep,
        locked_trash,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_rating_filter(4, false);

    assert_eq!(visible_indices(&controller), vec![1]);
}

#[test]
fn keep_three_filter_excludes_locked_keep_rows() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    controller
        .set_wav_entries_for_tests(vec![sample_entry("keep3.wav", Rating::KEEP_3), locked_keep]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_rating_filter(3, false);
    assert_eq!(visible_indices(&controller), vec![0]);

    controller.set_browser_rating_filter(4, true);
    assert_eq!(visible_indices(&controller), vec![0, 1]);
}

#[test]
fn invert_browser_rating_filter_selects_every_level_except_clicked_trash_chip() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);

    controller.invert_browser_rating_filter(-2);

    assert_eq!(
        controller
            .ui
            .browser
            .search
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -1, 0, 1, 2, 3, 4]
    );
    assert_eq!(visible_indices(&controller), vec![0, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn invert_browser_rating_filter_selects_every_level_except_clicked_neutral_chip() {
    let (mut controller, _source) = browser_rating_filter_fixture(true);

    controller.invert_browser_rating_filter(0);

    assert_eq!(
        controller
            .ui
            .browser
            .search
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 1, 2, 3, 4]
    );
    assert_eq!(visible_indices(&controller), vec![0, 1, 2, 4, 5, 6, 7]);
}

#[test]
fn browser_playback_age_filter_limits_visible_rows_and_composes_with_other_filters() {
    let (mut controller, source) = browser_rating_filter_fixture(false);
    let now_unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let kick_never = sample_entry("kick_never.wav", Rating::KEEP_1);
    let mut kick_month = sample_entry("kick_month.wav", Rating::KEEP_1);
    let mut kick_week = sample_entry("kick_week.wav", Rating::NEUTRAL);
    let mut hat_fresh = sample_entry("hat_fresh.wav", Rating::KEEP_1);
    kick_month.last_played_at = Some(now_unix_secs.saturating_sub(40 * 24 * 60 * 60));
    kick_week.last_played_at = Some(now_unix_secs.saturating_sub(10 * 24 * 60 * 60));
    hat_fresh.last_played_at = Some(now_unix_secs.saturating_sub(2 * 24 * 60 * 60));
    controller.set_wav_entries_for_tests(vec![kick_never, kick_month, kick_week, hat_fresh]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_playback_age_filter(
        crate::app::state::PlaybackAgeFilterChip::NeverPlayed,
        false,
    );
    assert_eq!(visible_indices(&controller), vec![0]);

    controller.set_browser_playback_age_filter(
        crate::app::state::PlaybackAgeFilterChip::OlderThanMonth,
        true,
    );
    assert_eq!(visible_indices(&controller), vec![0, 1]);

    controller.set_browser_rating_filter(1, false);
    assert_eq!(visible_indices(&controller), vec![0, 1]);

    controller.set_browser_search("kick");
    assert_eq!(visible_indices(&controller), vec![0, 1]);

    controller
        .ui
        .browser
        .marks
        .toggle_paths(&source.id, &[std::path::PathBuf::from("kick_month.wav")]);
    controller.toggle_browser_marked_filter();
    assert_eq!(visible_indices(&controller), vec![1]);
}

#[test]
fn invert_browser_playback_age_filter_selects_other_buckets_and_reclick_clears() {
    let (mut controller, _source) = browser_rating_filter_fixture(false);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("never.wav", Rating::NEUTRAL),
        sample_entry("month.wav", Rating::NEUTRAL),
        sample_entry("week.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.invert_browser_playback_age_filter(
        crate::app::state::PlaybackAgeFilterChip::OlderThanWeek,
    );

    assert_eq!(
        controller
            .ui
            .browser
            .search
            .playback_age_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![
            crate::app::state::PlaybackAgeFilterChip::NeverPlayed,
            crate::app::state::PlaybackAgeFilterChip::OlderThanMonth,
        ]
    );

    controller.invert_browser_playback_age_filter(
        crate::app::state::PlaybackAgeFilterChip::OlderThanWeek,
    );

    assert!(controller.ui.browser.search.playback_age_filter.is_empty());
}

#[test]
fn browser_search_limits_visible_rows() {
    let (mut controller, _source) = browser_rating_filter_fixture(false);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("kick.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("snare.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("hat.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_search("snr");

    assert_eq!(visible_indices(&controller), vec![1]);
}

#[test]
fn browser_search_orders_results_by_score_then_index() {
    let (mut controller, _source) = browser_rating_filter_fixture(false);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("abc.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("abc_extra.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("abdc.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_search("abc");

    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
}
