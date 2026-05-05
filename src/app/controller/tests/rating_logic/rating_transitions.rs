use super::*;

#[test]
fn adjust_rating_skips_neutral_from_rated() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    set_single_selected_entry(&mut controller, "keep1.wav", Rating::KEEP_1);
    controller.adjust_selected_rating(-1);
    assert_eq!(
        persisted_row(&mut controller, &source, "keep1.wav").tag,
        Rating::TRASH_1
    );

    set_single_selected_entry(&mut controller, "trash1.wav", Rating::TRASH_1);
    controller.adjust_selected_rating(1);
    assert_eq!(
        persisted_row(&mut controller, &source, "trash1.wav").tag,
        Rating::KEEP_1
    );

    set_single_selected_entry(&mut controller, "neutral_inc.wav", Rating::NEUTRAL);
    controller.adjust_selected_rating(1);
    assert_eq!(
        persisted_row(&mut controller, &source, "neutral_inc.wav").tag,
        Rating::KEEP_1
    );

    set_single_selected_entry(&mut controller, "neutral_dec.wav", Rating::NEUTRAL);
    controller.adjust_selected_rating(-1);
    assert_eq!(
        persisted_row(&mut controller, &source, "neutral_dec.wav").tag,
        Rating::TRASH_1
    );
}

#[test]
fn browser_target_rating_steps_without_dropping_back_to_neutral() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    set_single_selected_entry(&mut controller, "keep3.wav", Rating::KEEP_3);
    controller.tag_selected_browser_target(crate::app_core::state::BrowserTagTarget::Trash);
    assert_eq!(
        persisted_row(&mut controller, &source, "keep3.wav").tag,
        Rating::new(2)
    );

    set_single_selected_entry(&mut controller, "keep1.wav", Rating::KEEP_1);
    controller.tag_selected_browser_target(crate::app_core::state::BrowserTagTarget::Trash);
    assert_eq!(
        persisted_row(&mut controller, &source, "keep1.wav").tag,
        Rating::TRASH_1
    );

    set_single_selected_entry(&mut controller, "neutral.wav", Rating::NEUTRAL);
    controller.tag_selected_browser_target(crate::app_core::state::BrowserTagTarget::Keep);
    assert_eq!(
        persisted_row(&mut controller, &source, "neutral.wav").tag,
        Rating::KEEP_1
    );
}

#[test]
fn fourth_keep_rating_locks_sample_and_blocks_future_rating_changes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.set_wav_entries_for_tests(vec![sample_entry("keep3.wav", Rating::KEEP_3)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);

    controller.adjust_selected_rating(1);

    let entry = controller
        .wav_entry(0)
        .expect("locked sample should stay loaded");
    assert_eq!(entry.tag, Rating::KEEP_3);
    assert!(entry.locked);
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .locked_for_path(Path::new("keep3.wav"))
            .unwrap(),
        Some(true)
    );

    controller.adjust_selected_rating(-1);
    controller.tag_selected(Rating::NEUTRAL);

    let entry = controller
        .wav_entry(0)
        .expect("locked sample should stay loaded");
    assert_eq!(entry.tag, Rating::KEEP_3);
    assert!(entry.locked);
    assert_eq!(
        controller
            .database_for(&source)
            .unwrap()
            .locked_for_path(Path::new("keep3.wav"))
            .unwrap(),
        Some(true)
    );
}
