use super::*;
use crate::app::controller::library::wav_io;
use std::path::Path;

/// Normalize a wav file and return updated metadata + tag.
pub(crate) fn normalize_and_save_for_path(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
    absolute_path: &Path,
) -> Result<(u64, i64, crate::sample_sources::Rating), String> {
    let (mut samples, spec) = wav_io::read_samples_for_normalization(absolute_path)?;
    if samples.is_empty() {
        return Err("No audio data to normalize".into());
    }
    crate::analysis::audio::normalize_peak_in_place(&mut samples);

    let target_spec = hound::WavSpec {
        channels: spec.channels.max(1),
        sample_rate: spec.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    wav_io::write_normalized_wav(absolute_path, &samples, target_spec)?;

    let (file_size, modified_ns) = wav_io::file_metadata(absolute_path)?;
    let tag = controller.sample_tag_for(source, relative_path)?;
    Ok((file_size, modified_ns, tag))
}

/// Persist a rename or path change in the per-source database.
pub(crate) fn rewrite_db_entry_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    old_relative: &Path,
    new_relative: &Path,
    file_size: u64,
    modified_ns: i64,
    tag: crate::sample_sources::Rating,
) -> Result<(), String> {
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let last_played_at = db
        .last_played_at_for_path(old_relative)
        .map_err(|err| format!("Failed to load playback age: {err}"))?;
    let looped = db
        .looped_for_path(old_relative)
        .map_err(|err| format!("Failed to load loop marker: {err}"))?
        .unwrap_or(false);
    let mut batch = db
        .write_batch()
        .map_err(|err| format!("Failed to start database update: {err}"))?;
    batch
        .remove_file(old_relative)
        .map_err(|err| format!("Failed to drop old entry: {err}"))?;
    batch
        .upsert_file(new_relative, file_size, modified_ns)
        .map_err(|err| format!("Failed to register renamed file: {err}"))?;
    batch
        .set_tag(new_relative, tag)
        .map_err(|err| format!("Failed to copy tag: {err}"))?;
    batch
        .set_looped(new_relative, looped)
        .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
    if let Some(last_played_at) = last_played_at {
        batch
            .set_last_played_at(new_relative, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    batch
        .commit()
        .map_err(|err| format!("Failed to save rename: {err}"))
}

/// Upsert file metadata into the source database.
pub(crate) fn upsert_metadata_for_source(
    controller: &mut AppController,
    source: &SampleSource,
    relative_path: &Path,
    file_size: u64,
    modified_ns: i64,
) -> Result<(), String> {
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.upsert_file(relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to refresh metadata: {err}"))
}
