//! Analysis and persistence execution for supervisor-owned readiness stages.

use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use rusqlite::OptionalExtension;

mod analysis_decode;
mod backfill;
mod errors;
pub(crate) mod storage;
mod support;

const FEATURE_RMS_INDEX: usize = 2;

use self::{storage as db, support::now_epoch_seconds};

/// Typed failure emitted while producing one readiness stage.
#[derive(Debug)]
pub enum ReadinessStageError {
    /// The decoder identified a media-specific failure.
    Decode(wavecrate_analysis::AnalysisDecodeError),
    /// A non-decoder stage failure whose owner has no narrower type yet.
    Other(String),
}

impl From<String> for ReadinessStageError {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}

/// Produce current feature artifacts for one readiness-owned file target.
pub fn run_feature_stage(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, ReadinessStageError> {
    run_feature_stage_with_post_decode_hook(
        conn,
        source_root,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
        cancel,
        || {},
    )
}

#[allow(clippy::too_many_arguments)]
fn run_feature_stage_with_post_decode_hook(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
    post_decode_hook: impl FnOnce(),
) -> Result<bool, ReadinessStageError> {
    checkpoint(cancel, "feature analysis cancelled")?;
    let absolute_path = source_root.join(relative_path);
    let sample_id = db::build_sample_id(source_id, relative_path);
    if !ensure_current_sample_row(conn, &sample_id, relative_path, content_hash, source_root)? {
        return Ok(false);
    }
    if db::sample_content_hash(conn, &sample_id)?.as_deref() != Some(content_hash) {
        return Ok(false);
    }
    if !file_content_hash_matches(&absolute_path, content_hash, cancel)? {
        return Ok(false);
    }
    if let Some(cached) = db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )? {
        return Ok(materialize_cached_features(
            conn,
            source_root,
            &sample_id,
            content_hash,
            analysis_version,
            &cached,
        )?);
    }
    let decoded = analysis_decode::decode_for_readiness(&absolute_path)?;
    post_decode_hook();
    checkpoint(cancel, "feature analysis cancelled before computation")?;
    let features = wavecrate_analysis::compute_feature_vector_v1_for_decoded_audio(&decoded)?;
    checkpoint(cancel, "feature analysis cancelled before publication")?;
    let feature_blob = wavecrate_analysis::vector::encode_f32_le_blob(&features);
    let light_dsp_blob = wavecrate_analysis::light_dsp_from_features_v1(&features)
        .map(|values| wavecrate_analysis::vector::encode_f32_le_blob(&values));
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    if !file_content_hash_matches(&absolute_path, content_hash, cancel)? {
        return Ok(false);
    }
    let computed_at = now_epoch_seconds();
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_feature_publish")
        .map_err(|error| format!("Failed to start readiness feature transaction: {error}"))?;
    if db::sample_content_hash(&tx, &sample_id)?.as_deref() != Some(content_hash)
        || !manifest_content_hash_is_current(&tx, relative_path, content_hash)?
    {
        db::telemetry::commit_transaction(tx, "readiness_feature_publish_stale")
            .map_err(|error| format!("Failed to commit stale feature skip: {error}"))?;
        return Ok(false);
    }
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id: &sample_id,
            content_hash: Some(content_hash),
            duration_seconds: decoded.duration_seconds,
            sr_used: decoded.sample_rate_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        &sample_id,
        &feature_blob,
        light_dsp_blob.as_deref(),
        rms,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
        computed_at,
    )?;
    db::upsert_cached_features(
        &tx,
        db::CachedFeaturesUpsert {
            content_hash,
            analysis_version,
            feat_version: wavecrate_analysis::vector::FEATURE_VERSION_V1,
            vec_blob: &feature_blob,
            light_dsp_blob: light_dsp_blob.as_deref(),
            rms,
            computed_at,
            duration_seconds: decoded.duration_seconds,
            sr_used: decoded.sample_rate_used,
        },
    )?;
    db::telemetry::commit_transaction(tx, "readiness_feature_publish")
        .map_err(|error| format!("Failed to commit readiness features: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    checkpoint(cancel, "feature analysis cancelled after publication")?;
    Ok(true)
}

fn file_content_hash_matches(
    path: &Path,
    claimed_hash: &str,
    cancel: &AtomicBool,
) -> Result<bool, ReadinessStageError> {
    let mut file = File::open(path)
        .map_err(|error| format!("Failed to open readiness input {}: {error}", path.display()))?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        checkpoint(
            cancel,
            "feature analysis cancelled while verifying content hash",
        )?;
        let read = file.read(&mut buffer).map_err(|error| {
            format!("Failed to hash readiness input {}: {error}", path.display())
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().as_str() == claimed_hash)
}

fn manifest_content_hash_is_current(
    conn: &rusqlite::Connection,
    relative_path: &Path,
    content_hash: &str,
) -> Result<bool, String> {
    let relative_path = relative_path.to_string_lossy().replace('\\', "/");
    conn.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM wav_files
            WHERE path = ?1 AND content_hash = ?2 AND missing = 0
         )",
        rusqlite::params![relative_path, content_hash],
        |row| row.get(0),
    )
    .map_err(|error| format!("Failed to revalidate readiness sample manifest: {error}"))
}

fn materialize_cached_features(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    sample_id: &str,
    content_hash: &str,
    analysis_version: &str,
    cached: &db::CachedFeatures,
) -> Result<bool, String> {
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_feature_cache_apply")
        .map_err(|error| format!("Failed to start readiness feature cache transaction: {error}"))?;
    if db::sample_content_hash(&tx, sample_id)?.as_deref() != Some(content_hash) {
        db::telemetry::commit_transaction(tx, "readiness_feature_cache_stale")
            .map_err(|error| format!("Failed to commit stale feature cache skip: {error}"))?;
        return Ok(false);
    }
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id,
            content_hash: Some(content_hash),
            duration_seconds: cached.duration_seconds,
            sr_used: cached.sr_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        sample_id,
        &cached.vec_blob,
        cached.light_dsp_blob.as_deref(),
        cached.rms,
        cached.feat_version,
        cached.computed_at,
    )?;
    db::telemetry::commit_transaction(tx, "readiness_feature_cache_apply")
        .map_err(|error| format!("Failed to commit cached readiness features: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    Ok(true)
}

fn ensure_current_sample_row(
    conn: &mut rusqlite::Connection,
    sample_id: &str,
    relative_path: &Path,
    content_hash: &str,
    source_root: &Path,
) -> Result<bool, String> {
    let relative_path = relative_path.to_string_lossy().replace('\\', "/");
    let manifest = conn
        .query_row(
            "SELECT file_size, modified_ns
             FROM wav_files
             WHERE path = ?1 AND content_hash = ?2 AND missing = 0",
            rusqlite::params![relative_path, content_hash],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| format!("Failed to read readiness sample manifest: {error}"))?;
    let Some((size, modified_ns)) = manifest else {
        return Ok(false);
    };
    let size = u64::try_from(size)
        .map_err(|_| format!("Readiness sample has a negative file size: {relative_path}"))?;
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_sample_upsert")
        .map_err(|error| format!("Failed to start readiness sample transaction: {error}"))?;
    db::upsert_samples_in_tx(
        &tx,
        &[db::SampleMetadata {
            sample_id: sample_id.to_string(),
            content_hash: content_hash.to_string(),
            size,
            mtime_ns: modified_ns,
        }],
    )?;
    db::telemetry::commit_transaction(tx, "readiness_sample_upsert")
        .map_err(|error| format!("Failed to commit readiness sample metadata: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    Ok(true)
}

/// Produce current embedding and aspect artifacts for one readiness-owned file target.
pub fn run_embedding_stage(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    checkpoint(cancel, "embedding analysis cancelled")?;
    let sample_id = db::build_sample_id(source_id, relative_path);
    if db::sample_content_hash(conn, &sample_id)?.as_deref() != Some(content_hash) {
        return Ok(false);
    }
    if db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )?
    .is_none()
    {
        return Ok(false);
    }
    backfill::run_readiness_embedding_backfill(
        conn,
        source_root,
        &[sample_id.clone()],
        wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
        analysis_version,
        Some(cancel),
        Some(1),
    )?;
    checkpoint(cancel, "embedding analysis cancelled after publication")?;
    let embedding = db::cached_embedding_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
    )?;
    let aspects = db::cached_aspect_descriptors_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    )?;
    Ok(embedding.is_some() && aspects.is_some())
}

fn checkpoint(cancel: &AtomicBool, reason: &'static str) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err(reason.to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample_sources::{SampleSource, SourceDatabase, SourceId};

    #[test]
    fn feature_publication_rejects_file_mutated_while_stage_is_in_flight() {
        let directory = tempfile::tempdir().expect("temporary readiness source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("feature-content-fence"),
            directory.path().to_path_buf(),
        );
        let relative_path = Path::new("sample.wav");
        let absolute_path = source.root.join(relative_path);
        write_wav(&absolute_path, 440);
        let original_bytes = std::fs::read(&absolute_path).expect("read original wav");
        let original_hash = blake3::hash(&original_bytes).to_hex().to_string();
        let metadata = std::fs::metadata(&absolute_path).expect("read original metadata");
        let mut connection = SourceDatabase::open_connection_for_background_job(&source.root)
            .expect("open source db");
        connection
            .execute(
                "INSERT INTO wav_files (
                    path, file_size, modified_ns, content_hash, extension, missing, file_identity
                 ) VALUES (?1, ?2, 1, ?3, 'wav', 0, 'feature-content-fence-identity')",
                rusqlite::params![
                    relative_path.to_string_lossy(),
                    i64::try_from(metadata.len()).expect("wav size fits i64"),
                    original_hash,
                ],
            )
            .expect("seed source manifest");
        let cancel = AtomicBool::new(false);

        let published = run_feature_stage_with_post_decode_hook(
            &mut connection,
            &source.root,
            source.id.as_str(),
            relative_path,
            &original_hash,
            wavecrate_analysis::analysis_version(),
            &cancel,
            || write_wav(&absolute_path, 880),
        )
        .expect("run fenced feature stage");

        assert!(!published, "changed bytes must make the claim retryable");
        let cached: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM analysis_cache_features WHERE content_hash = ?1",
                [&original_hash],
                |row| row.get(0),
            )
            .expect("count cached features");
        assert_eq!(cached, 0, "changed bytes must not poison the content cache");
        assert_ne!(
            blake3::hash(&std::fs::read(&absolute_path).expect("read mutated wav"))
                .to_hex()
                .as_str(),
            original_hash
        );
    }

    fn write_wav(path: &Path, frequency_hz: usize) {
        let sample_rate = 8_000_u32;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create test wav");
        for sample_index in 0..usize::try_from(sample_rate).expect("sample rate fits usize") {
            let phase = sample_index as f32 * frequency_hz as f32 / sample_rate as f32;
            let sample = ((phase * std::f32::consts::TAU).sin() * 12_000.0) as i16;
            writer.write_sample(sample).expect("write wav sample");
        }
        writer.finalize().expect("finalize test wav");
    }
}
