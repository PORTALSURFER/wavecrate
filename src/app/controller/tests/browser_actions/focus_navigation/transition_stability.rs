use super::*;
use crate::app_core::ui_projection::{
    project_app_model, project_browser_panel_frame_model, project_waveform_model,
};

fn seed_loaded_browser_focus(
    controller: &mut crate::app::controller::AppController,
    relative_path: &str,
) {
    controller.focus_browser_row_only(0);
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from(relative_path));
    controller.ui.loaded_wav = Some(PathBuf::from(relative_path));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;
}

fn project_transition_state(
    controller: &mut crate::app::controller::AppController,
) -> (Option<String>, Option<String>, bool, bool) {
    let browser = project_browser_panel_frame_model(controller);
    let waveform = project_waveform_model(controller);
    let app = project_app_model(controller);
    (
        browser.focused_sample_label,
        waveform.loaded_label,
        waveform.loading,
        app.transport_running,
    )
}

#[test]
fn browser_focus_transition_preview_keeps_latest_waveform_target_visible() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    seed_loaded_browser_focus(&mut controller, "one.wav");

    controller.apply_ui_action(NativeUiAction::FocusBrowserRow { visible_row: 1 });
    controller.apply_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
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

    let (browser_label, waveform_label, waveform_loading, transport_running) =
        project_transition_state(&mut controller);
    assert_eq!(browser_label.as_deref(), Some("three"));
    assert_eq!(waveform_label.as_deref(), Some("one"));
    assert!(waveform_loading);
    assert!(!transport_running);
}

#[test]
fn browser_focus_transition_commit_keeps_browser_and_waveform_targets_aligned() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    seed_loaded_browser_focus(&mut controller, "one.wav");

    controller.apply_ui_action(NativeUiAction::FocusBrowserRow { visible_row: 1 });
    controller.apply_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });
    controller.apply_ui_action(NativeUiAction::CommitFocusedBrowserRow);

    assert!(controller.runtime.browser.selection_transition.is_some());
    controller.prepare_ui_frame(false);

    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
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
    assert!(controller.runtime.jobs.pending_playback.is_none());

    let (browser_label, waveform_label, waveform_loading, transport_running) =
        project_transition_state(&mut controller);
    assert_eq!(browser_label.as_deref(), Some("three"));
    assert_eq!(waveform_label.as_deref(), Some("one"));
    assert!(waveform_loading);
    assert!(!transport_running);
}

#[test]
fn browser_focus_transition_superseded_commit_clears_stale_loading_before_projection() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    seed_loaded_browser_focus(&mut controller, "one.wav");

    controller.apply_ui_action(NativeUiAction::FocusBrowserRow { visible_row: 1 });
    controller.apply_ui_action(NativeUiAction::CommitFocusedBrowserRow);
    controller.apply_ui_action(NativeUiAction::FocusBrowserRow { visible_row: 0 });
    controller.prepare_ui_frame(false);

    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("one.wav"))
    );

    let (browser_label, waveform_label, waveform_loading, transport_running) =
        project_transition_state(&mut controller);
    assert_eq!(browser_label.as_deref(), Some("one"));
    assert_eq!(waveform_label.as_deref(), Some("one"));
    assert!(waveform_loading);
    assert!(!transport_running);
}
