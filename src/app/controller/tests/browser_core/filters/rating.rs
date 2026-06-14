use super::*;

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
