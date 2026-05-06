use super::super::*;

impl From<runtime_contract::BrowserTriageTarget> for BrowserTagTarget {
    fn from(value: runtime_contract::BrowserTriageTarget) -> Self {
        match value {
            runtime_contract::BrowserTriageTarget::Negative => Self::Trash,
            runtime_contract::BrowserTriageTarget::Neutral => Self::Neutral,
            runtime_contract::BrowserTriageTarget::Positive => Self::Keep,
        }
    }
}

impl From<BrowserTagTarget> for runtime_contract::BrowserTriageTarget {
    fn from(value: BrowserTagTarget) -> Self {
        match value {
            BrowserTagTarget::Trash => Self::Negative,
            BrowserTagTarget::Neutral => Self::Neutral,
            BrowserTagTarget::Keep => Self::Positive,
        }
    }
}
