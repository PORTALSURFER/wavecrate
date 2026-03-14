use super::*;

#[test]
fn sample_browser_indices_track_tags() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash.wav", crate::sample_sources::Rating::TRASH_3),
        sample_entry("neutral.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("keep.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
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

    let selected = controller.ui.browser.selected.unwrap();
    assert_eq!(selected.column, TriageFlagColumn::Neutral);
    assert_eq!(controller.ui.browser.selected_visible, Some(1));
    let loaded = controller.ui.browser.loaded.unwrap();
    assert_eq!(loaded.column, TriageFlagColumn::Keep);
    assert_eq!(controller.ui.browser.loaded_visible, Some(2));
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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash3.wav", Rating::TRASH_3),
        sample_entry("trash2.wav", Rating::new(-2)),
        sample_entry("trash1.wav", Rating::TRASH_1),
        sample_entry("neutral.wav", Rating::NEUTRAL),
        sample_entry("keep1.wav", Rating::KEEP_1),
        sample_entry("keep2.wav", Rating::new(2)),
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash3.wav", Rating::TRASH_3),
        sample_entry("trash2.wav", Rating::new(-2)),
        sample_entry("trash1.wav", Rating::TRASH_1),
        sample_entry("neutral.wav", Rating::NEUTRAL),
        sample_entry("keep1.wav", Rating::KEEP_1),
        sample_entry("keep2.wav", Rating::new(2)),
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.invert_browser_rating_filter(4);

    assert_eq!(
        controller
            .ui
            .browser
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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash3.wav", Rating::TRASH_3),
        sample_entry("trash2.wav", Rating::new(-2)),
        sample_entry("trash1.wav", Rating::TRASH_1),
        sample_entry("neutral.wav", Rating::NEUTRAL),
        sample_entry("keep1.wav", Rating::KEEP_1),
        sample_entry("keep2.wav", Rating::new(2)),
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.invert_browser_rating_filter(4);
    assert_eq!(
        controller
            .ui
            .browser
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 0, 1, 2, 3]
    );

    controller.invert_browser_rating_filter(4);

    assert!(controller.ui.browser.rating_filter.is_empty());
    assert_eq!(visible_indices(&controller), vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn locked_keep_filter_only_matches_locked_keep_rows() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash3.wav", Rating::TRASH_3),
        sample_entry("trash2.wav", Rating::new(-2)),
        sample_entry("trash1.wav", Rating::TRASH_1),
        sample_entry("neutral.wav", Rating::NEUTRAL),
        sample_entry("keep1.wav", Rating::KEEP_1),
        sample_entry("keep2.wav", Rating::new(2)),
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.invert_browser_rating_filter(-2);

    assert_eq!(
        controller
            .ui
            .browser
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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    let mut locked_keep = sample_entry("locked_keep.wav", Rating::KEEP_3);
    locked_keep.locked = true;
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash3.wav", Rating::TRASH_3),
        sample_entry("trash2.wav", Rating::new(-2)),
        sample_entry("trash1.wav", Rating::TRASH_1),
        sample_entry("neutral.wav", Rating::NEUTRAL),
        sample_entry("keep1.wav", Rating::KEEP_1),
        sample_entry("keep2.wav", Rating::new(2)),
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.invert_browser_rating_filter(0);

    assert_eq!(
        controller
            .ui
            .browser
            .rating_filter
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![-3, -2, -1, 1, 2, 3, 4]
    );
    assert_eq!(visible_indices(&controller), vec![0, 1, 2, 4, 5, 6, 7]);
}

#[test]
fn browser_search_limits_visible_rows() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
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
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
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
