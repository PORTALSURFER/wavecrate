use super::*;
use crate::app::controller::library::wav_io::file_metadata;
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};

/// Disk output information for one generated loop-crossfade copy.
pub(super) struct LoopCrossfadeFileOutput {
    /// Relative path inserted into the source database and browser.
    pub relative_path: PathBuf,
    /// Absolute path written on disk for the generated file.
    pub absolute_path: PathBuf,
}

/// Write the rendered crossfade output to the next available file path.
pub(super) fn write_loop_crossfade_copy(
    root: &Path,
    relative_path: &Path,
    rendered: &audio::RenderedLoopCrossfade,
) -> Result<LoopCrossfadeFileOutput, String> {
    let relative_path =
        next_loop_crossfade_relative_path(relative_path, root, rendered.suffix.as_str());
    let absolute_path = root.join(&relative_path);
    write_loop_crossfade_wav(&absolute_path, &rendered.samples, rendered.spec)?;
    Ok(LoopCrossfadeFileOutput {
        relative_path,
        absolute_path,
    })
}

/// Register the generated copy in the source database, browser cache, and similarity queue.
pub(super) fn register_loop_crossfade_entry(
    controller: &mut AppController,
    source: &SampleSource,
    output: &LoopCrossfadeFileOutput,
    tag: Rating,
) -> Result<(), String> {
    let (file_size, modified_ns) = file_metadata(&output.absolute_path)?;
    let db = controller
        .database_for(source)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    db.upsert_file(&output.relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to sync database entry: {err}"))?;
    db.set_tag(&output.relative_path, tag)
        .map_err(|err| format!("Failed to sync tag: {err}"))?;
    controller.insert_cached_entry(
        source,
        WavEntry {
            relative_path: output.relative_path.clone(),
            file_size,
            modified_ns,
            content_hash: None,
            tag,
            looped: false,
            locked: false,
            missing: false,
            last_played_at: None,
        },
    );
    controller.enqueue_similarity_for_new_sample(
        source,
        &output.relative_path,
        file_size,
        modified_ns,
    );
    Ok(())
}

/// Resolve the next collision-free relative output path for the rewritten sample.
fn next_loop_crossfade_relative_path(relative_path: &Path, root: &Path, suffix: &str) -> PathBuf {
    let parent = relative_path.parent().unwrap_or_else(|| Path::new(""));
    let stem = relative_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("sample");
    let ext = relative_path.extension().and_then(|ext| ext.to_str());
    let mut counter = 0;
    loop {
        let name = if counter == 0 {
            match ext {
                Some(ext) => format!("{stem}_{suffix}.{ext}"),
                None => format!("{stem}_{suffix}"),
            }
        } else {
            match ext {
                Some(ext) => format!("{stem}_{suffix}_{counter}.{ext}"),
                None => format!("{stem}_{suffix}_{counter}"),
            }
        };
        let candidate = parent.join(name);
        if !root.join(&candidate).exists() {
            return candidate;
        }
        counter += 1;
    }
}

/// Persist one floating-point WAV payload to disk.
fn write_loop_crossfade_wav(
    path: &Path,
    samples: &[f32],
    spec: hound::WavSpec,
) -> Result<(), String> {
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|err| format!("Failed to write wav: {err}"))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|err| format!("Failed to write sample: {err}"))?;
    }
    writer
        .finalize()
        .map_err(|err| format!("Failed to finalize wav: {err}"))
}
