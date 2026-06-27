use super::gui_state_for_span_tests;
use crate::native_app::test_support::state::GuiMessage;
use radiant::prelude::{self as ui, IntoView};

#[test]
fn transaction_group_undoes_and_redoes_as_one_entry() {
    let mut state = gui_state_for_span_tests();
    state.audio.volume = 0.1;
    state.begin_transaction("Grouped edit");
    state.register_transaction_action(
        "first",
        |transaction| {
            transaction.set_audio_volume(0.1);
            Ok(())
        },
        |transaction| {
            transaction.set_audio_volume(0.4);
            Ok(())
        },
    );
    state.register_transaction_action(
        "second",
        |transaction| {
            transaction.set_audio_volume(0.4);
            Ok(())
        },
        |transaction| {
            transaction.set_audio_volume(0.8);
            Ok(())
        },
    );
    assert!(state.commit_transaction());

    state.audio.volume = 0.8;
    state.undo_transaction();
    assert_eq!(state.ui.status.sample, "Undid Grouped edit");
    assert_eq!(state.audio.volume, 0.1);
    state.redo_transaction();
    assert_eq!(state.ui.status.sample, "Redid Grouped edit");
    assert_eq!(state.audio.volume, 0.8);
}

#[test]
fn transaction_list_modal_open_close_updates_chrome_state() {
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();

    assert!(!state.ui.chrome.transaction_list_open);
    state.apply_message(GuiMessage::ToggleTransactionList, &mut context);
    assert!(state.ui.chrome.transaction_list_open);

    state.apply_message(GuiMessage::CloseTransactionList, &mut context);
    assert!(!state.ui.chrome.transaction_list_open);
}

#[test]
fn transaction_list_target_undo_and_redo_walk_through_selected_row() {
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();
    state.audio.volume = 0.3;
    state.register_transaction_action(
        "First volume",
        |transaction| {
            transaction.set_audio_volume(0.0);
            Ok(())
        },
        |transaction| {
            transaction.set_audio_volume(0.1);
            Ok(())
        },
    );
    state.register_transaction_action(
        "Second volume",
        |transaction| {
            transaction.set_audio_volume(0.1);
            Ok(())
        },
        |transaction| {
            transaction.set_audio_volume(0.2);
            Ok(())
        },
    );
    state.register_transaction_action(
        "Third volume",
        |transaction| {
            transaction.set_audio_volume(0.2);
            Ok(())
        },
        |transaction| {
            transaction.set_audio_volume(0.3);
            Ok(())
        },
    );

    state.apply_message(GuiMessage::UndoTransactionsThrough(2), &mut context);

    assert_eq!(state.audio.volume, 0.1);
    assert_eq!(state.ui.status.sample, "Undid 2 through Second volume");

    state.apply_message(GuiMessage::RedoTransactionsThrough(3), &mut context);

    assert_eq!(state.audio.volume, 0.3);
    assert_eq!(state.ui.status.sample, "Redid 2 through Third volume");
}

#[test]
fn transaction_list_modal_renders_registered_transactions() {
    let mut state = gui_state_for_span_tests();
    state.ui.chrome.transaction_list_open = true;
    state.register_transaction_action("Rename sample", |_| Ok(()), |_| Ok(()));
    state.begin_transaction("Open batch");
    state.register_transaction_action("First action", |_| Ok(()), |_| Ok(()));

    let frame = crate::native_app::test_support::state::view(&mut state)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Transactions"));
    assert!(frame.paint_plan.contains_text("Rename sample"));
    assert!(frame.paint_plan.contains_text("Open batch"));
    assert!(frame.paint_plan.contains_text("Open"));
    assert!(frame.paint_plan.contains_text("Undo"));
}
