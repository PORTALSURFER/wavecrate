use super::{NormalizationJob, NormalizationResult};
use crate::app::controller::library::wav_io;

/// Execute one normalization job and return the controller-facing result payload.
pub(super) fn run_normalization_job(job: NormalizationJob) -> NormalizationResult {
    let source_id = job.source.id.clone();
    let relative_path = job.relative_path.clone();

    let result = (|| {
        let backup =
            crate::app::controller::undo::OverwriteBackup::capture_before(&job.absolute_path)?;
        let (mut samples, spec) = wav_io::read_samples_for_normalization(&job.absolute_path)?;
        if samples.is_empty() {
            return Err("No audio data to normalize".to_string());
        }

        crate::analysis::audio::normalize_peak_in_place(&mut samples);

        let target_spec = hound::WavSpec {
            channels: spec.channels.max(1),
            sample_rate: spec.sample_rate.max(1),
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        wav_io::write_normalized_wav(&job.absolute_path, &samples, target_spec)?;

        let (file_size, modified_ns) = wav_io::file_metadata(&job.absolute_path)?;

        let db = job
            .source
            .open_db()
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let tag = db
            .tag_for_path(&job.relative_path)
            .map_err(|err| format!("Failed to read database: {err}"))?
            .ok_or_else(|| "Sample not found in database".to_string())?;

        backup.capture_after(&job.absolute_path)?;
        Ok((file_size, modified_ns, tag, backup))
    })();

    NormalizationResult {
        source_id,
        relative_path,
        result,
    }
}
