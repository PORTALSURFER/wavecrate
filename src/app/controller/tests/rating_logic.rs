use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::state::history::RandomHistoryEntry;
use crate::app::state::TriageFlagFilter;
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[test]
fn adjust_rating_skips_neutral_from_rated() {
    // Setup - we need to mock the selection
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    // Helper macro to setup a single file and select it
    macro_rules! setup_file {
        ($name:expr, $rating:expr) => {
            let entry = sample_entry($name, $rating);
            controller.set_wav_entries_for_tests(vec![entry]);
            controller.rebuild_wav_lookup();
            controller.rebuild_browser_lists();
            // Select the row
            controller.sample_view.wav.selected_wav = Some(PathBuf::from($name));
        };
    }

    // Helper to find row
    let find_row = |rows: &[crate::sample_sources::WavEntry], name: &str| {
        rows.iter()
            .find(|r| r.relative_path.to_string_lossy() == name)
            .expect("file not found")
            .clone()
    };

    // Case 1: Keep 1 -> Decrement -> Trash 1 (skip Neutral)
    setup_file!("keep1.wav", Rating::KEEP_1);
    controller.adjust_selected_rating(-1);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(
        find_row(&rows, "keep1.wav").tag,
        Rating::TRASH_1,
        "Decreasing Keep 1 should go to Trash 1"
    );

    // Case 2: Trash 1 -> Increment -> Keep 1 (skip Neutral)
    setup_file!("trash1.wav", Rating::TRASH_1);
    controller.adjust_selected_rating(1);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(
        find_row(&rows, "trash1.wav").tag,
        Rating::KEEP_1,
        "Increasing Trash 1 should go to Keep 1"
    );

    // Case 3: Neutral -> Increment -> Keep 1 (Normal behavior)
    setup_file!("neutral_inc.wav", Rating::NEUTRAL);
    controller.adjust_selected_rating(1);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(
        find_row(&rows, "neutral_inc.wav").tag,
        Rating::KEEP_1,
        "Increasing Neutral should go to Keep 1"
    );

    // Case 4: Neutral -> Decrement -> Trash 1 (Normal behavior)
    setup_file!("neutral_dec.wav", Rating::NEUTRAL);
    controller.adjust_selected_rating(-1);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(
        find_row(&rows, "neutral_dec.wav").tag,
        Rating::TRASH_1,
        "Decreasing Neutral should go to Trash 1"
    );
}

#[test]
fn advance_after_rating_respects_random_navigation() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);

    controller.ui.browser.search.random_navigation_mode = true;
    controller.settings.controls.advance_after_rating = true;

    let id = source.id.clone();
    controller
        .history
        .random_history
        .mark_played(&id, &PathBuf::from("a.wav"));
    controller
        .history
        .random_history
        .mark_played(&id, &PathBuf::from("b.wav"));

    controller.focus_browser_row(0);
    assert_eq!(controller.selected_row_index(), Some(0));

    controller.adjust_selected_rating(1);

    let selected_path = controller
        .sample_view
        .wav
        .selected_wav
        .as_ref()
        .expect("selection");
    assert_eq!(selected_path, &PathBuf::from("c.wav"));
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("c.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("c.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("c.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());

    for _ in 0..50 {
        controller.poll_background_jobs();
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(Path::new("c.wav"))
            && controller.ui.waveform.loading.is_none()
            && controller.ui.waveform.image.is_some()
        {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("c.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.ui.waveform.image.is_some());
}

#[test]
fn rating_previous_random_history_entry_restores_waveform_for_replacement() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);

    controller.settings.controls.advance_after_rating = true;
    controller.settings.feature_flags.autoplay_selection = false;
    controller.set_browser_rating_filter(0, false);
    controller.toggle_random_navigation_mode();

    let source_id = source.id.clone();
    controller
        .history
        .random_history
        .mark_played(&source_id, Path::new("b.wav"));
    controller
        .history
        .random_history
        .mark_played(&source_id, Path::new("c.wav"));
    controller
        .history
        .random_history
        .entries
        .push_back(RandomHistoryEntry {
            source_id: source_id.clone(),
            relative_path: PathBuf::from("b.wav"),
        });
    controller
        .history
        .random_history
        .entries
        .push_back(RandomHistoryEntry {
            source_id,
            relative_path: PathBuf::from("c.wav"),
        });
    controller.history.random_history.cursor = Some(1);

    controller.play_previous_random_sample();

    for _ in 0..50 {
        controller.poll_background_jobs();
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(Path::new("b.wav"))
            && controller.ui.waveform.loading.is_none()
            && controller.ui.waveform.image.is_some()
        {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("b.wav"))
    );
    assert!(controller.ui.waveform.image.is_some());

    controller.adjust_selected_rating(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());

    for _ in 0..50 {
        controller.poll_background_jobs();
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(Path::new("a.wav"))
            && controller.ui.waveform.loading.is_none()
            && controller.ui.waveform.image.is_some()
        {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.ui.waveform.image.is_some());
}

/// Browser keep/trash actions should step across zero without returning to neutral.
#[test]
fn browser_target_rating_steps_without_dropping_back_to_neutral() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());

    macro_rules! setup_file {
        ($name:expr, $rating:expr) => {
            let entry = sample_entry($name, $rating);
            controller.set_wav_entries_for_tests(vec![entry]);
            controller.rebuild_wav_lookup();
            controller.rebuild_browser_lists();
            controller.sample_view.wav.selected_wav = Some(PathBuf::from($name));
        };
    }

    let find_row = |rows: &[crate::sample_sources::WavEntry], name: &str| {
        rows.iter()
            .find(|r| r.relative_path.to_string_lossy() == name)
            .expect("file not found")
            .clone()
    };

    setup_file!("keep3.wav", Rating::KEEP_3);
    controller.tag_selected_browser_target(crate::app_core::state::BrowserTagTarget::Trash);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(find_row(&rows, "keep3.wav").tag, Rating::new(2));

    setup_file!("keep1.wav", Rating::KEEP_1);
    controller.tag_selected_browser_target(crate::app_core::state::BrowserTagTarget::Trash);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(find_row(&rows, "keep1.wav").tag, Rating::TRASH_1);

    setup_file!("neutral.wav", Rating::NEUTRAL);
    controller.tag_selected_browser_target(crate::app_core::state::BrowserTagTarget::Keep);
    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    assert_eq!(find_row(&rows, "neutral.wav").tag, Rating::KEEP_1);
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
            .locked_for_path(std::path::Path::new("keep3.wav"))
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
            .locked_for_path(std::path::Path::new("keep3.wav"))
            .unwrap(),
        Some(true)
    );
}

#[test]
fn undo_adjust_rating_refocuses_original_sample_under_filter() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.set_browser_filter(TriageFlagFilter::Untagged);

    controller.focus_browser_row_only(1);
    controller.adjust_selected_rating(1);
    assert_eq!(
        controller.visible_row_for_path(std::path::Path::new("two.wav")),
        None
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(std::path::Path::new("three.wav"))
    );

    controller.undo();

    assert_eq!(
        controller.visible_row_for_path(std::path::Path::new("two.wav")),
        Some(1)
    );
    assert_eq!(
        controller.wav_entry(1).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(std::path::Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
}
