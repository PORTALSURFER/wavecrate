//! Entry acceptance policy for the retained browser-search filter stage.

use super::super::super::folders::filter_accepts_tag;
use super::super::super::*;
use std::path::Path;

pub(super) fn entry_accepted_by_job(
    job: &SearchJob,
    entry: &CompactSearchEntry,
    relative_path: &Path,
    bpm: Option<f32>,
) -> bool {
    filter_accepts_tag(
        job.filter,
        &job.rating_filter,
        &job.playback_age_filter,
        job.tag_named_filter,
        entry.tag_named,
        entry.tag,
        entry.locked,
        entry.last_played_at,
        job.playback_age_now_unix_secs,
        &job.sidebar_filters,
        relative_path,
        bpm,
    )
}
