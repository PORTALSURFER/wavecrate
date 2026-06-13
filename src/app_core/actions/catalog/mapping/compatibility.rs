use super::shared::{GuiActionKind, Kind, NativeCompatibilityAction};

pub(super) fn compatibility_action_kind(action: &NativeCompatibilityAction) -> GuiActionKind {
    match action {
        NativeCompatibilityAction::Undo => Kind::Undo,
        NativeCompatibilityAction::Redo => Kind::Redo,
        NativeCompatibilityAction::CheckForUpdates => Kind::CheckForUpdates,
        NativeCompatibilityAction::OpenUpdateLink => Kind::OpenUpdateLink,
        NativeCompatibilityAction::InstallUpdate => Kind::InstallUpdate,
        NativeCompatibilityAction::DismissUpdate => Kind::DismissUpdate,
        NativeCompatibilityAction::SelectColumn { .. } => Kind::SelectColumn,
        NativeCompatibilityAction::MoveColumn { .. } => Kind::MoveColumn,
        NativeCompatibilityAction::SeekWaveform { .. } => Kind::SeekWaveform,
        NativeCompatibilityAction::SetWaveformCursor { .. } => Kind::SetWaveformCursor,
    }
}
