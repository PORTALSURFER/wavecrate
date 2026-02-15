/// Status tone variants used to indicate operation outcome in the status bar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StatusTone {
    /// Idle/neutral status.
    Idle,
    /// Busy/working status.
    Busy,
    /// Informational status.
    Info,
    /// Warning status.
    Warning,
    /// Error status.
    Error,
}

/// Status badge + text shown in the footer.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusBarState {
    /// Main status message text.
    pub text: String,
    /// Current status tone used to format the status badge.
    pub status_tone: StatusTone,
    /// Rolling status log entries.
    pub log: Vec<String>,
}

impl StatusBarState {
    /// Default status shown when no source is selected.
    pub fn idle() -> Self {
        Self {
            text: "Add a sample source to get started".into(),
            status_tone: StatusTone::Idle,
            log: Vec::new(),
        }
    }

    /// Concatenate log entries into a single displayable string.
    pub fn log_text(&self) -> String {
        if self.log.is_empty() {
            return String::new();
        }
        self.log.join("\n")
    }
}
