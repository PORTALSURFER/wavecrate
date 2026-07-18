use crate::app::controller::StatusTone;

#[derive(Clone, Debug)]
pub(crate) enum StatusMessage {
    SelectSourceFirst { tone: StatusTone },
    SelectSourceToScan,
    ScanAlreadyRunning,
    RandomHistoryEmpty,
    RandomHistoryStart,
    RandomNavOff,
    NoSamplesToRandomize,
    AddSourceFirst { tone: StatusTone },
    AddSourceWithSamplesFirst,
    Custom { text: String, tone: StatusTone },
}

impl StatusMessage {
    pub(crate) fn custom(text: impl Into<String>, tone: StatusTone) -> Self {
        Self::Custom {
            text: text.into(),
            tone,
        }
    }

    pub(crate) fn into_text_and_tone(self) -> (String, StatusTone) {
        match self {
            StatusMessage::SelectSourceFirst { tone } => ("Select a source first".into(), tone),
            StatusMessage::SelectSourceToScan => {
                ("Select a source to scan".into(), StatusTone::Warning)
            }
            StatusMessage::ScanAlreadyRunning => {
                ("Scan already in progress".into(), StatusTone::Info)
            }
            StatusMessage::RandomHistoryEmpty => ("No random history yet".into(), StatusTone::Info),
            StatusMessage::RandomHistoryStart => {
                ("Reached start of random history".into(), StatusTone::Info)
            }
            StatusMessage::RandomNavOff => ("Random navigation off".into(), StatusTone::Info),
            StatusMessage::NoSamplesToRandomize => {
                ("No samples available to randomize".into(), StatusTone::Info)
            }
            StatusMessage::AddSourceFirst { tone } => ("Add a source first".into(), tone),
            StatusMessage::AddSourceWithSamplesFirst => {
                ("Add a source with samples first".into(), StatusTone::Info)
            }
            StatusMessage::Custom { text, tone } => (text, tone),
        }
    }
}
