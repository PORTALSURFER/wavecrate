//! Focus-transition orchestration for the sample browser.
//!
//! The public `AppController` entrypoints stay stable, but the implementation
//! is split by responsibility so preview navigation, commit-time loading, and
//! post-mutation review follow-up can evolve independently.

use super::*;
use crate::app::controller::StatusTone;
use crate::app::state::FocusContext;
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use std::path::{Path, PathBuf};

mod commit;
mod preview;
mod review_follow_up;
mod shared;

/// Follow-up plans for browser review actions that mutate the focused sample.
#[derive(Clone)]
pub(crate) enum BrowserReviewFollowUpPlan {
    /// Move away from the current row using the next visible-browser target.
    AdvanceFromPrimaryRow { primary_row: usize },
    /// Reuse the already-refocused replacement sample after a filter removes the current row.
    UseFocusedReplacement,
    /// Focus one explicit target path instead of relying on pre-mutation row state.
    FocusPath {
        /// Source-relative path to focus next.
        path: PathBuf,
        /// Random-navigation source to record when the path came from a random jump.
        random_history_source_id: Option<SourceId>,
    },
}

/// Linear-mode loading policy for browser review follow-up navigation.
#[derive(Clone, Copy)]
pub(crate) enum BrowserReviewLinearMode {
    /// Commit the focused/next row using the standard selection load pipeline.
    Commit,
    /// Preview the focused/next row immediately so playback/waveform loading starts at once.
    Preview,
}
