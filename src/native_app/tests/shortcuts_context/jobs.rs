use crate::native_app::test_support::state::{GuiMessage, NativeAppState, default_gui_shortcuts};
use radiant::prelude as ui;

fn state_with_job_details_open() -> NativeAppState {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.job_details_open = true;
    state
}

#[test]
fn job_details_escape_shortcut_closes_the_popover() {
    let state = state_with_job_details_open();

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::CloseJobDetails));
    assert!(resolution.handled);
}

#[test]
fn job_details_popover_keeps_default_shortcuts_active() {
    let state = state_with_job_details_open();

    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::CloseBracket));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::AdjustSelectedRatingWithoutAdvance(1))
    );
    assert!(resolution.handled);
}

#[test]
fn job_details_popover_keeps_navigation_shortcuts_active() {
    let state = state_with_job_details_open();

    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::ArrowDown));

    assert_eq!(
        resolution.action,
        Some(GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
        })
    );
    assert!(resolution.handled);
}
