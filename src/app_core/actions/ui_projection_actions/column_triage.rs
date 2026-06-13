use serde::{Deserialize, Serialize};

/// Browser column triage actions owned by the current app-core action model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColumnTriageAction {
    SelectColumn { index: usize },
    MoveColumn { delta: i8 },
}
