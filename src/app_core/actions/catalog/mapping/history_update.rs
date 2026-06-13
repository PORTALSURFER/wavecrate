use super::shared::{GuiActionKind, Kind, NativeHistoryUpdateAction};

pub(super) fn history_update_action_kind(action: &NativeHistoryUpdateAction) -> GuiActionKind {
    match action {
        NativeHistoryUpdateAction::Undo => Kind::Undo,
        NativeHistoryUpdateAction::Redo => Kind::Redo,
        NativeHistoryUpdateAction::CheckForUpdates => Kind::CheckForUpdates,
        NativeHistoryUpdateAction::OpenUpdateLink => Kind::OpenUpdateLink,
        NativeHistoryUpdateAction::InstallUpdate => Kind::InstallUpdate,
        NativeHistoryUpdateAction::DismissUpdate => Kind::DismissUpdate,
    }
}
