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
