use super::gui_state_for_span_tests;
use crate::native_app::{
    test_support::state::{GuiMessage, WaveformInteraction},
    waveform::{WaveformEditFadeHandle, WaveformEditFadeOuterGainHandle},
};
use radiant::prelude::{self as ui, IntoView};
use wavecrate::selection::SelectionRange;

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
fn waveform_fade_drag_registers_one_transaction() {
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();
    let before = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    state.waveform.current.set_edit_selection_range(before);

    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::BeginEditFade {
            handle: WaveformEditFadeHandle::OutStart,
            visible_ratio: 0.5,
        }),
        &mut context,
    );
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.45,
        }),
        &mut context,
    );
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::FinishSelection {
            visible_ratio: 0.45,
        }),
        &mut context,
    );

    let after = state
        .waveform
        .current
        .edit_selection()
        .expect("edit selection after fade drag");
    assert_ne!(after, before);
    let items = state.transactions.history.list_items();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "Waveform fade");
    assert_eq!(items[0].action_labels, vec![String::from("Waveform fade")]);

    state.apply_message(GuiMessage::UndoTransaction, &mut context);
    assert_eq!(state.waveform.current.edit_selection(), Some(before));
    assert_eq!(state.ui.status.sample, "Undid Waveform fade");

    state.apply_message(GuiMessage::RedoTransaction, &mut context);
    assert_eq!(state.waveform.current.edit_selection(), Some(after));
    assert_eq!(state.ui.status.sample, "Redid Waveform fade");
}

#[test]
fn waveform_fade_outer_gain_drag_registers_one_transaction() {
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();
    let before = SelectionRange::new(0.2, 0.6)
        .with_fade_in(0.25, 0.2)
        .with_fade_in_mute(0.2)
        .with_fade_in_outer_gain(0.25);
    state.waveform.current.set_edit_selection_range(before);

    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::BeginEditFadeOuterGain {
            handle: WaveformEditFadeOuterGainHandle::In,
            vertical_ratio: 0.25,
        }),
        &mut context,
    );
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::UpdateEditFadeOuterGain {
            vertical_ratio: 0.5,
        }),
        &mut context,
    );
    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::FinishEditFadeOuterGain {
            vertical_ratio: 0.5,
        }),
        &mut context,
    );

    let after = state
        .waveform
        .current
        .edit_selection()
        .expect("edit selection after outer gain drag");
    assert_ne!(after, before);
    assert_eq!(state.transactions.history.list_items().len(), 1);

    state.apply_message(GuiMessage::UndoTransaction, &mut context);
    assert_eq!(state.waveform.current.edit_selection(), Some(before));

    state.apply_message(GuiMessage::RedoTransaction, &mut context);
    assert_eq!(state.waveform.current.edit_selection(), Some(after));
}

#[test]
fn no_op_waveform_fade_clear_silence_does_not_register_transaction() {
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();
    let selection = SelectionRange::new(0.2, 0.6).with_fade_out(0.25, 0.2);
    state.waveform.current.set_edit_selection_range(selection);

    state.apply_message(
        GuiMessage::Waveform(WaveformInteraction::ClearEditFadeSilence {
            handle: WaveformEditFadeHandle::OutOuterEnd,
        }),
        &mut context,
    );

    assert_eq!(state.waveform.current.edit_selection(), Some(selection));
    assert!(state.transactions.history.list_items().is_empty());
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
