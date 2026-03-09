use super::*;
use crate::sample_sources::Rating;

/// Parameters that identify which selection span to export and how to register it.
pub(crate) struct SelectionClipExportRequest<'a> {
    /// Source that owns the loaded audio.
    pub source_id: &'a SourceId,
    /// Relative path of the loaded audio inside the source.
    pub relative_path: &'a Path,
    /// Normalized bounds to crop from the loaded audio.
    pub bounds: SelectionRange,
    /// Optional rating assigned to the exported clip.
    pub target_tag: Option<Rating>,
    /// Whether the exported clip should appear in the visible browser.
    pub add_to_browser: bool,
    /// Whether the exported clip should be inserted into the source database.
    pub register_in_source: bool,
}

/// Parameters for registering a newly written selection clip in caches and databases.
pub(crate) struct SelectionEntryRecordRequest<'a> {
    /// Source that owns the written clip.
    pub source: &'a SampleSource,
    /// Relative path of the written clip.
    pub relative_path: PathBuf,
    /// Optional rating assigned during export.
    pub target_tag: Option<Rating>,
    /// Whether the browser should surface the new clip immediately.
    pub add_to_browser: bool,
    /// Whether the source DB should be updated for the new clip.
    pub register_in_source: bool,
    /// Whether loop metadata should be persisted for the clip.
    pub looped: bool,
    /// Optional BPM metadata to persist when `looped` is true.
    pub bpm: Option<f32>,
}
