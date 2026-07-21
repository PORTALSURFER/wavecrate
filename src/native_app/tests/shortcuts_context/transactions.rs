use crate::native_app::{
    test_support::state::{
        GuiMessage, NativeAppState, NativeAppStateFixture, default_gui_shortcuts,
    },
    waveform::PlaymarkLabelMessage,
};
use radiant::prelude as ui;

fn transaction_list_shortcut() -> ui::KeyPress {
    ui::KeyPress {
        key: ui::KeyCode::Backslash,
        command: true,
        control: false,
        shift: true,
        alt: false,
    }
}

#[test]
fn command_shift_backslash_shortcut_toggles_transaction_list() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution = default_gui_shortcuts(&state).resolve(transaction_list_shortcut());

    assert_eq!(resolution.action, Some(GuiMessage::ToggleTransactionList));
    assert!(resolution.handled);
}

#[test]
fn transaction_list_modal_escape_closes_transaction_list() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.transaction_list_open = true;

    let resolution = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));

    assert_eq!(resolution.action, Some(GuiMessage::CloseTransactionList));
    assert!(resolution.handled);
}

#[test]
fn playmark_label_editor_owns_escape_and_blocks_transport_shortcuts() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    state.waveform.current.set_play_selection_range(0.2, 0.4);
    assert!(state.waveform.current.begin_playmark_label_edit(false, 4));

    let escape = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Escape));
    let space = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::Space));

    assert_eq!(
        escape.action,
        Some(GuiMessage::PlaymarkLabel(PlaymarkLabelMessage::Cancel))
    );
    assert!(escape.handled);
    assert_eq!(space.action, None);
    assert!(space.handled);
}

#[test]
fn command_undo_redo_shortcuts_route_to_transactions() {
    let state = NativeAppState::load_default().expect("default state loads");

    let undo = default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::Z));
    let redo_shift_z = default_gui_shortcuts(&state).resolve(ui::KeyPress {
        key: ui::KeyCode::Z,
        command: true,
        control: false,
        shift: true,
        alt: false,
    });
    let redo_y = default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::Y));

    assert_eq!(undo.action, Some(GuiMessage::UndoTransaction));
    assert_eq!(redo_shift_z.action, Some(GuiMessage::RedoTransaction));
    assert_eq!(redo_y.action, Some(GuiMessage::RedoTransaction));
    assert!(undo.handled);
    assert!(redo_shift_z.handled);
    assert!(redo_y.handled);
}

#[test]
fn bracket_shortcuts_route_to_rating_adjustments() {
    let state = NativeAppState::load_default().expect("default state loads");

    let down = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::OpenBracket));
    let up = default_gui_shortcuts(&state).resolve(ui::KeyPress::new(ui::KeyCode::CloseBracket));

    assert_eq!(
        down.action,
        Some(GuiMessage::AdjustSelectedRatingWithoutAdvance(-1))
    );
    assert_eq!(
        up.action,
        Some(GuiMessage::AdjustSelectedRatingWithoutAdvance(1))
    );
    assert!(down.handled);
    assert!(up.handled);
}
