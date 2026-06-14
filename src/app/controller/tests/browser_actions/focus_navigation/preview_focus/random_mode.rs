use super::super::*;

#[test]
fn native_move_browser_focus_uses_random_mode_pool_without_repeating_current_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.focus_browser_row_only(0);
    controller.toggle_random_navigation_mode();
    controller.mark_random_navigation_path_for_current_list(&source.id, Path::new("two.wav"));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.apply_ui_action(NativeUiAction::Browser(
        crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { delta: 1 },
    ));

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("three.wav"))
    );
}
