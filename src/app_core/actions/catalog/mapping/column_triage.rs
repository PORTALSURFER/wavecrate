use super::shared::{GuiActionKind, Kind, NativeColumnTriageAction};

pub(super) fn column_triage_action_kind(action: &NativeColumnTriageAction) -> GuiActionKind {
    match action {
        NativeColumnTriageAction::SelectColumn { .. } => Kind::SelectColumn,
        NativeColumnTriageAction::MoveColumn { .. } => Kind::MoveColumn,
    }
}
