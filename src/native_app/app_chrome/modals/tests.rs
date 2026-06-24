use super::projection::TransactionListProjection;
use crate::native_app::test_support::state::NativeAppState;
use radiant::prelude::{self as ui, IntoView};

#[test]
fn transaction_list_projection_formats_summary_and_rows() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    let empty = TransactionListProjection::from_state(&state);
    assert_eq!(empty.summary, "no undo | no redo | closed");
    assert!(empty.rows.is_empty());

    state.register_transaction_action("Rename sample", |_| Ok(()), |_| Ok(()));
    state.begin_transaction("Open batch");
    state.register_transaction_action("First action", |_| Ok(()), |_| Ok(()));

    let projection = TransactionListProjection::from_state(&state);
    assert_eq!(
        projection.summary,
        "undo ready | no redo | open transaction"
    );
    assert_eq!(projection.rows.len(), 2);
    assert_eq!(projection.rows[0].label, "Open batch");
    assert_eq!(projection.rows[0].action_summary, "1 action: First action");
    assert_eq!(projection.rows[0].state.label(), "Open");
    assert_eq!(projection.rows[1].label, "Rename sample");
    assert_eq!(projection.rows[1].action_summary, "1 action: Rename sample");
    assert_eq!(projection.rows[1].state.label(), "Undo");
}

#[test]
fn transaction_list_modal_uses_registered_modal_identity() {
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.ui.chrome.transaction_list_open = true;

    let frame = crate::native_app::app_chrome::modals::transaction_list(&state)
        .view_frame_at_size_with_default_theme(ui::Vector2::new(520.0, 360.0));

    assert!(
        frame
            .layout
            .rects
            .contains_key(&super::identity::TRANSACTION_LIST_MODAL_ID),
        "transaction list modal should keep the registered automation/test id"
    );
}
