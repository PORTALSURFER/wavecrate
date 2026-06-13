use super::*;
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
    let cached_before = playback_age_cache_token(
        &controller,
        &controller.ui.browser.search.playback_age_filter,
    );

    let _ = super::visible_rows::build_visible_rows_with_now(&mut controller, None, None, now + 1);
    assert_eq!(
        playback_age_cache_token(
            &controller,
            &controller.ui.browser.search.playback_age_filter
        ),
        cached_before
    );

    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanMonth);
    let widened_filter = controller.ui.browser.search.playback_age_filter.clone();
    let _ = super::visible_rows::build_visible_rows_with_now(&mut controller, None, None, now + 1);
    assert_ne!(
        playback_age_cache_token(&controller, &widened_filter),
        cached_before
    );
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .playback_age_token_caches
            .len(),
        2
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
        playback_age_cache_token(
            &controller,
            &controller.ui.browser.search.playback_age_filter
        )
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
            .playback_age_token_caches
            .is_empty()
    );

    let _ = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        now_unix_secs,
    );
    assert_eq!(
        playback_age_cache_token(
            &controller,
            &controller.ui.browser.search.playback_age_filter
        )
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
