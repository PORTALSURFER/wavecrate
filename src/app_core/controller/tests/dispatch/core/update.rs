use super::*;

#[test]
fn apply_ui_action_routes_update_status_case() {
    let mut controller = controller_for_grouped_dispatch();

    controller.apply_ui_action(NativeUiAction::CheckForUpdates);

    assert_eq!(controller.ui.update.status, UpdateStatus::Checking);
}
