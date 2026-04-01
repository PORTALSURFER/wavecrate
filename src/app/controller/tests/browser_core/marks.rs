use super::*;
use crate::sample_sources::SampleSource;

#[test]
fn browser_sample_mark_toggle_marks_and_unmarks_focused_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert!(!controller.browser_sample_marked(&source.id, Path::new("one.wav")));

    controller.toggle_browser_sample_mark();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("two.wav")));
}

#[test]
fn browser_sample_mark_toggle_applies_to_selection_and_focused_row() {
    let (mut controller, source) = browser_mark_fixture();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert!(!controller.browser_sample_marked(&source.id, Path::new("three.wav")));
}

#[test]
fn focused_browser_mark_advances_and_previews_next_sample() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());

    wait_for_waveform_image(&mut controller, Path::new("two.wav"));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_some());
}

#[test]
fn marked_filter_composes_with_rating_search_and_folder_filters() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::create_dir_all(source.root.join("drums")).expect("create drums folder");
    std::fs::create_dir_all(source.root.join("fx")).expect("create fx folder");
    controller.set_wav_entries_for_tests(vec![
        sample_entry("drums/kick_marked.wav", Rating::KEEP_1),
        sample_entry("drums/snare_marked.wav", Rating::NEUTRAL),
        sample_entry("fx/kick_marked.wav", Rating::KEEP_1),
        sample_entry("drums/kick_unmarked.wav", Rating::KEEP_1),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(2);
    controller.toggle_browser_sample_mark();

    controller.refresh_folder_browser_for_tests();
    let drums_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("drums"))
        .expect("expected drums folder");
    controller.replace_folder_selection(drums_index);
    controller.set_browser_search("kick");
    controller.set_browser_rating_filter(1, false);
    controller.toggle_browser_marked_filter();

    assert!(controller.ui.browser.search.marked_only);
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("drums/kick_marked.wav")]
    );
    Ok(())
}

#[test]
fn unmarking_focused_marked_row_under_marked_filter_refocuses_next_visible_row() {
    let (mut controller, source) = browser_mark_fixture();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    controller.toggle_browser_marked_filter();

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("two.wav")]
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn focused_browser_mark_uses_random_preview_follow_up() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;
    controller.ui.browser.search.random_navigation_mode = true;
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("one.wav"));
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("two.wav"));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("three.wav"))
    );
}

#[test]
fn marked_filter_mark_review_uses_random_follow_up_when_random_mode_is_enabled() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(2);
    controller.toggle_browser_sample_mark();
    controller.toggle_browser_marked_filter();
    controller.ui.browser.search.random_navigation_mode = true;
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("one.wav"));
    controller
        .history
        .random_history
        .mark_played(&source.id, &PathBuf::from("two.wav"));

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("two.wav"), PathBuf::from("three.wav")]
    );
    assert_eq!(
        controller.focused_browser_path().as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("three.wav"))
    );
}

#[test]
fn unmarking_focused_marked_row_under_marked_filter_previews_replacement_sample() {
    let (mut controller, source) = browser_mark_fixture();
    write_browser_mark_wavs(&source.root);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    controller.toggle_browser_marked_filter();
    controller.focus_browser_row_only(0);

    controller.toggle_browser_sample_mark();

    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("two.wav")]
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn multi_selection_mark_does_not_auto_advance() {
    let (mut controller, source) = browser_mark_fixture();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert!(controller.ui.waveform.loading.is_none());
}

#[test]
fn browser_sample_marks_survive_source_switches_within_session() {
    let (mut controller, source_a) = dummy_controller();
    let source_b = SampleSource::new(source_a.root.parent().unwrap().join("source_b"));
    std::fs::create_dir_all(&source_b.root).expect("create second source root");
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.selection_state.ctx.selected_source = Some(source_a.id.clone());

    controller.set_wav_entries_for_tests(vec![sample_entry("a.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    controller.select_source(Some(source_b.id.clone()));
    controller.set_wav_entries_for_tests(vec![sample_entry("b.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source_a.id, Path::new("a.wav")));
    assert!(controller.browser_sample_marked(&source_b.id, Path::new("b.wav")));
    assert_eq!(controller.ui.browser.marks.marked_paths.len(), 2);
}

#[test]
fn browser_sample_marks_follow_renames_and_prune_deleted_entries() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("old.wav", Rating::NEUTRAL),
        sample_entry("keep.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.update_selection_paths(&source, Path::new("old.wav"), Path::new("renamed.wav"));
    controller.set_wav_entries_for_tests(vec![
        sample_entry("renamed.wav", Rating::NEUTRAL),
        sample_entry("keep.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("old.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("renamed.wav")));

    controller.set_wav_entries_for_tests(vec![sample_entry("keep.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("renamed.wav")));
}
