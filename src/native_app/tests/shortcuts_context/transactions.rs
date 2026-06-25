use crate::native_app::test_support::state::{GuiMessage, NativeAppState, default_gui_shortcuts};
use radiant::prelude as ui;

#[test]
fn shift_u_shortcut_toggles_transaction_list() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_shift(ui::KeyCode::U));

    assert_eq!(resolution.action, Some(GuiMessage::ToggleTransactionList));
    assert!(resolution.handled);
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

    assert_eq!(down.action, Some(GuiMessage::AdjustSelectedRating(-1)));
    assert_eq!(up.action, Some(GuiMessage::AdjustSelectedRating(1)));
    assert!(down.handled);
    assert!(up.handled);
}
