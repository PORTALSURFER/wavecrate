use super::*;

/// Probe and store missing duration metadata for samples in a source.
/// Returns the number of samples updated.
pub(crate) fn update_missing_durations_for_source(
    source: &crate::sample_sources::SampleSource,
) -> Result<usize, String> {
    let mut conn = db::open_source_db(&source.root)?;
    let staged_samples = scan::stage_samples_for_source(source, true)?;
    update_missing_sample_durations(&mut conn, source, &staged_samples)
}

pub(super) fn update_missing_sample_durations(
    conn: &mut rusqlite::Connection,
    source: &crate::sample_sources::SampleSource,
    samples: &[db::SampleMetadata],
) -> Result<usize, String> {
    if samples.is_empty() {
        return Ok(0);
    }
    let sample_ids: Vec<String> = samples
        .iter()
        .map(|sample| sample.sample_id.clone())
        .collect();
    let missing_ids = db::sample_ids_missing_duration(conn, &sample_ids)?;
    if missing_ids.is_empty() {
        return Ok(0);
    }
    let mut updated = 0usize;
    for sample in samples {
        if !missing_ids.contains(&sample.sample_id) {
            continue;
        }
        let (_source_id, relative_path) = match db::parse_sample_id(&sample.sample_id) {
            Ok(parsed) => parsed,
            Err(err) => {
                warn!("Skipping duration probe for {}: {err}", sample.sample_id);
                continue;
            }
        };
        let absolute = source.root.join(&relative_path);
        let probe = match wavecrate_analysis::probe_metadata(&absolute) {
            Ok(probe) => probe,
            Err(err) => {
                warn!("Failed to probe duration for {}: {err}", absolute.display());
                continue;
            }
        };
        let Some(duration_seconds) = probe
            .duration_seconds
            .filter(|duration| duration.is_finite() && *duration > 0.0)
        else {
            continue;
        };
        let sample_rate = probe
            .sample_rate
            .unwrap_or(wavecrate_analysis::ANALYSIS_SAMPLE_RATE)
            .max(1);
        match db::update_sample_duration(conn, &sample.sample_id, duration_seconds, sample_rate) {
            Ok(true) => updated += 1,
            Ok(false) => {}
            Err(err) => {
                warn!("Failed to store duration for {}: {err}", sample.sample_id);
            }
        }
    }
    Ok(updated)
}
