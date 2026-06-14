use serde::{Deserialize, Serialize};

/// History and update actions emitted by UI runtime surfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryUpdateAction {
    /// Undo the latest undoable edit.
    Undo,
    /// Redo the latest undone edit.
    Redo,
    /// Check for an available Wavecrate update.
    CheckForUpdates,
    /// Open the currently available update link.
    OpenUpdateLink,
    /// Install the downloaded update and exit.
    InstallUpdate,
    /// Dismiss the current update notification.
    DismissUpdate,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{NativeRetainedUiAction, NativeUiAction};

    #[test]
    fn history_update_action_round_trips_through_current_action_contract() {
        let action = NativeUiAction::HistoryAndUpdate(HistoryUpdateAction::CheckForUpdates);
        let json = serde_json::to_value(&action).expect("serialize action");
        assert_eq!(
            json,
            serde_json::json!({ "HistoryAndUpdate": "CheckForUpdates" })
        );

        let parsed: NativeUiAction = serde_json::from_value(json).expect("parse action");
        assert_eq!(parsed, action);
    }

    #[test]
    fn retained_flat_update_payload_still_parses_for_compatibility() {
        let parsed: NativeRetainedUiAction =
            serde_json::from_value(serde_json::json!("CheckForUpdates"))
                .expect("parse retained flat action");

        assert_eq!(
            parsed.into_current(),
            NativeUiAction::HistoryAndUpdate(HistoryUpdateAction::CheckForUpdates)
        );
    }
}
