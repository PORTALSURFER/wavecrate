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

/// Owner for the currently visible footer status line.
///
/// Status writes always append to the rolling log. This owner only arbitrates
/// whether a write may replace the visible footer text while a higher-priority
/// narrative, such as an active file operation, is in progress.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StatusScope {
    /// Background maintenance or refresh work that should not clobber file ops.
    BackgroundMaintenance,
    /// Ordinary one-off messages and passive informational updates.
    Passive,
    /// User-initiated file operations whose progress/result should stay coherent.
    FileOp,
}

/// Status badge + text shown in the footer.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusBarState {
    /// Main status message text.
    pub text: String,
    /// Current status tone used to format the status badge.
    pub status_tone: StatusTone,
    /// Scope currently owning the visible status line.
    pub visible_scope: StatusScope,
    /// Rolling status log entries.
    pub log: Vec<String>,
}

impl StatusBarState {
    /// Default status shown when no source is selected.
    pub fn idle() -> Self {
        Self {
            text: "Add a sample source to get started".into(),
            status_tone: StatusTone::Idle,
            visible_scope: StatusScope::Passive,
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
