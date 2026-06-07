use super::gui_state_for_span_tests;
use radiant::prelude::{self as ui, IntoView};

#[test]
fn transaction_group_undoes_and_redoes_as_one_entry() {
    let mut state = gui_state_for_span_tests();
    state.volume = 0.1;
    state.begin_transaction("Grouped edit");
    state.register_transaction_action(
        "first",
        |state| {
            state.volume = 0.1;
            Ok(())
        },
        |state| {
            state.volume = 0.4;
            Ok(())
        },
    );
    state.register_transaction_action(
        "second",
        |state| {
            state.volume = 0.4;
            Ok(())
        },
        |state| {
            state.volume = 0.8;
            Ok(())
        },
    );
    assert!(state.commit_transaction());

    state.volume = 0.8;
    state.undo_transaction();
    assert_eq!(state.sample_status, "Undid Grouped edit");
    assert_eq!(state.volume, 0.1);
    state.redo_transaction();
    assert_eq!(state.sample_status, "Redid Grouped edit");
    assert_eq!(state.volume, 0.8);
}

#[test]
fn transaction_list_modal_renders_registered_transactions() {
    let mut state = gui_state_for_span_tests();
    state.transaction_list_open = true;
    state.register_transaction_action("Rename sample", |_| Ok(()), |_| Ok(()));
    state.begin_transaction("Open batch");
    state.register_transaction_action("First action", |_| Ok(()), |_| Ok(()));

    let frame = crate::native_app::view(&mut state)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(960.0, 540.0));

    assert!(frame.paint_plan.contains_text("Transactions"));
    assert!(frame.paint_plan.contains_text("Rename sample"));
    assert!(frame.paint_plan.contains_text("Open batch"));
    assert!(frame.paint_plan.contains_text("Open"));
    assert!(frame.paint_plan.contains_text("Undo"));
}
