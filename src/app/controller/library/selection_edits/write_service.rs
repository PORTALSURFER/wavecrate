use super::apply_short_edge_fades_to_clip;
use super::buffer::{
    SelectionEditBuffer, SelectionTarget, load_selection_buffer, write_selection_wav,
};
use super::ops::crop_buffer;
use crate::app::controller::library::wav_io::file_metadata;
use crate::sample_sources::{Rating, SourceDatabase, WavEntry};
use std::path::PathBuf;
use std::time::Duration;

/// Inputs for applying one destructive edit to an existing sample file.
pub(crate) struct SelectionEditWriteRequest<'a> {
    pub(crate) target: &'a SelectionTarget,
    pub(crate) db: &'a SourceDatabase,
    pub(crate) tag: Rating,
    pub(crate) last_played_at: Option<i64>,
    pub(crate) looped: bool,
}

/// Persisted metadata produced after rewriting one sample file in place.
pub(crate) struct SelectionEditWriteOutcome {
    pub(crate) entry: WavEntry,
}

/// Inputs for writing the cropped selection to a new sibling sample file.
pub(crate) struct CropNewSampleWriteRequest<'a> {
    pub(crate) target: &'a SelectionTarget,
    pub(crate) new_relative: PathBuf,
    pub(crate) db: &'a SourceDatabase,
    pub(crate) tag: Rating,
    pub(crate) edge_fade: Option<Duration>,
}

/// Persisted metadata produced after exporting one cropped sample file.
pub(crate) struct CropNewSampleWriteOutcome {
    pub(crate) entry: WavEntry,
    pub(crate) new_absolute: PathBuf,
}

/// Rewrite one existing sample file after applying `edit` to its loaded selection buffer.
pub(crate) fn apply_selection_edit_write<F>(
    request: SelectionEditWriteRequest<'_>,
    mut edit: F,
) -> Result<SelectionEditWriteOutcome, String>
where
    F: FnMut(&mut SelectionEditBuffer) -> Result<(), String>,
{
    let mut buffer =
        load_selection_buffer(&request.target.absolute_path, request.target.selection)?;
    edit(&mut buffer)?;
    if buffer.samples.is_empty() {
        return Err("No audio data after edit".into());
    }
    write_buffer_to_path(&request.target.absolute_path, &buffer)?;
    let entry = sync_sample_entry(
        request.db,
        &request.target.relative_path,
        &request.target.absolute_path,
        request.tag,
        request.last_played_at,
        request.looped,
    )?;
    Ok(SelectionEditWriteOutcome { entry })
}

/// Write the cropped selection to a new sibling file and synchronize its database row.
pub(crate) fn crop_selection_to_new_sample_write(
    request: CropNewSampleWriteRequest<'_>,
) -> Result<CropNewSampleWriteOutcome, String> {
    let new_absolute = request.target.source.root.join(&request.new_relative);
    let mut buffer =
        load_selection_buffer(&request.target.absolute_path, request.target.selection)?;
    crop_buffer(&mut buffer)?;
    if buffer.samples.is_empty() {
        return Err("Selection has no audio to crop".into());
    }
    if let Some(fade_duration) = request.edge_fade {
        apply_short_edge_fades_to_clip(
            &mut buffer.samples,
            buffer.channels,
            buffer.sample_rate,
            fade_duration,
        );
    }
    write_buffer_to_path(&new_absolute, &buffer)?;
    let entry = sync_sample_entry(
        request.db,
        &request.new_relative,
        &new_absolute,
        request.tag,
        None,
        false,
    )?;
    Ok(CropNewSampleWriteOutcome {
        entry,
        new_absolute,
    })
}

fn write_buffer_to_path(
    target: &std::path::Path,
    buffer: &SelectionEditBuffer,
) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: buffer.spec_channels,
        sample_rate: buffer.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    write_selection_wav(&target.to_path_buf(), &buffer.samples, spec)
}

fn sync_sample_entry(
    db: &SourceDatabase,
    relative_path: &std::path::Path,
    absolute_path: &std::path::Path,
    tag: Rating,
    last_played_at: Option<i64>,
    looped: bool,
) -> Result<WavEntry, String> {
    let (file_size, modified_ns) = file_metadata(absolute_path)?;
    db.upsert_file(relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    db.set_tag(relative_path, tag)
        .map_err(|err| format!("Failed to sync tag: {err}"))?;
    Ok(WavEntry {
        relative_path: relative_path.to_path_buf(),
        file_size,
        modified_ns,
        content_hash: None,
        tag,
        looped,
        locked: false,
        missing: false,
        last_played_at,
    })
}
