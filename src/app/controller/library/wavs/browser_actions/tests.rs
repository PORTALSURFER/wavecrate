use super::*;
use crate::app::controller::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::sample_sources::Rating;
use crate::selection::SelectionRange;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn browser_row_is_queued_or_loaded(controller: &AppController, relative_path: &Path) -> bool {
    controller
        .runtime
        .jobs
        .pending_audio
        .as_ref()
        .is_some_and(|pending| pending.relative_path == relative_path)
        || controller.ui.waveform.loading.as_deref() == Some(relative_path)
        || controller.sample_view.wav.loaded_wav.as_deref() == Some(relative_path)
}

#[test]
/// Preview intent should update focus without queueing heavy audio load work.
fn focus_browser_row_preview_is_load_free() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_row_with_intent(1, BrowserFocusIntent::Preview);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
}

#[test]
/// Commit intent should queue or apply loading for the newly focused sample.
fn focus_browser_row_commit_requests_load() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_row_with_intent(1, BrowserFocusIntent::Commit);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(browser_row_is_queued_or_loaded(
        &controller,
        Path::new("two.wav")
    ));
}

#[test]
/// Range extension should keep the original focus row as the anchor boundary.
fn extend_browser_selection_respects_anchor() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);

    controller.extend_browser_selection_to_row(2);

    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![
            PathBuf::from("one.wav"),
            PathBuf::from("two.wav"),
            PathBuf::from("three.wav")
        ]
    );
}

#[test]
/// Toggle selection should seed the anchor row into the multi-select set when toggling away from it.
fn toggle_browser_row_selection_preserves_anchor_membership() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);

    controller.toggle_browser_row_selection(2);

    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav"), PathBuf::from("three.wav")]
    );
}

#[test]
/// Selecting all rows should preserve the existing anchor while disabling autoscroll.
fn select_all_browser_rows_preserves_anchor_and_disables_autoscroll() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(1);

    controller.select_all_browser_rows();

    assert!(!controller.ui.browser.selection.autoscroll);
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![
            PathBuf::from("one.wav"),
            PathBuf::from("two.wav"),
            PathBuf::from("three.wav")
        ]
    );
}

#[test]
/// Preview-only row focus should preserve multi-selection while updating focus state.
fn focus_browser_row_only_preserves_multi_selection_membership() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.extend_browser_selection_to_row(2);

    controller.focus_browser_row_only(1);

    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![
            PathBuf::from("one.wav"),
            PathBuf::from("two.wav"),
            PathBuf::from("three.wav")
        ]
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.browser.selection.commit_focus_pending);
}

#[test]
/// Direct browser click playback should respect the active loop toggle.
fn focus_browser_row_and_play_uses_active_loop_state() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) =
        prepare_with_source_and_wav_entries(vec![sample_entry("one.wav", Rating::NEUTRAL)]);
    controller.audio.player = Some(Rc::new(RefCell::new(player)));
    controller.settings.feature_flags.autoplay_selection = false;
    load_waveform_selection(
        &mut controller,
        &source,
        "one.wav",
        &[0.0, 0.1, -0.1, 0.2],
        SelectionRange::new(0.0, 1.0),
    );
    controller.ui.waveform.loop_enabled = true;

    controller.focus_browser_row_and_play_action(0);

    assert!(controller.ui.waveform.loop_enabled);
    let player_ref = controller.audio.player.as_ref().expect("player").borrow();
    if !player_ref.is_playing() {
        return;
    }
    assert!(player_ref.is_looping());
}

#[test]
/// Direct browser click playback should keep selection responsive while loading catches up.
fn focus_browser_row_and_play_queues_latest_preview_for_unloaded_sample() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.ui.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;
    controller.ui.waveform.loading = None;

    controller.focus_browser_row_and_play_action(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(controller.sample_view.wav.loaded_wav, None);
    assert_eq!(controller.ui.loaded_wav, None);
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
}
