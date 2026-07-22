//! Analysis and persistence execution for supervisor-owned readiness stages.

use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

mod analysis_decode;
mod backfill;
mod errors;
pub(crate) mod storage;
mod support;

const FEATURE_RMS_INDEX: usize = 2;

/// CPU/IO output prepared without holding the source-database writer permit.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreparedFeatureStage {
    feature_version: i64,
    feature_blob: Vec<u8>,
    light_dsp_blob: Option<Vec<u8>>,
    rms: Option<f32>,
    computed_at: i64,
    duration_seconds: f32,
    sample_rate_used: u32,
}

/// Read-only input required to compute embedding and aspect payloads.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EmbeddingStageInput {
    features: Vec<f32>,
    computed_at: i64,
}

/// CPU output prepared without a source-database connection.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PreparedEmbeddingStage {
    embedding: Vec<f32>,
    aspect_blob: Vec<u8>,
    aspect_valid_mask: u32,
    computed_at: i64,
}

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
    let prepared = if let Some(cached) = db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )? {
        PreparedFeatureStage::from(cached)
    } else {
        let Some(prepared) = prepare_feature_stage_with_post_decode_hook(
            source_root,
            relative_path,
            content_hash,
            cancel,
            post_decode_hook,
        )?
        else {
            return Ok(false);
        };
        prepared
    };
    publish_feature_stage(
        conn,
        source_root,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
        &prepared,
    )
    .map_err(Into::into)
}

impl From<db::CachedFeatures> for PreparedFeatureStage {
    fn from(cached: db::CachedFeatures) -> Self {
        Self {
            feature_version: cached.feat_version,
            feature_blob: cached.vec_blob,
            light_dsp_blob: cached.light_dsp_blob,
            rms: cached.rms,
            computed_at: cached.computed_at,
            duration_seconds: cached.duration_seconds,
            sample_rate_used: cached.sr_used,
        }
    }
}

/// Load a reusable feature payload using read-only database access.
pub fn cached_feature_stage(
    conn: &rusqlite::Connection,
    content_hash: &str,
    analysis_version: &str,
) -> Result<Option<PreparedFeatureStage>, String> {
    db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )
    .map(|cached| cached.map(PreparedFeatureStage::from))
}

/// Decode and compute one feature payload without opening a database.
pub fn prepare_feature_stage(
    source_root: &Path,
    relative_path: &Path,
    content_hash: &str,
    cancel: &AtomicBool,
) -> Result<Option<PreparedFeatureStage>, ReadinessStageError> {
    prepare_feature_stage_with_post_decode_hook(
        source_root,
        relative_path,
        content_hash,
        cancel,
        || {},
    )
}

fn prepare_feature_stage_with_post_decode_hook(
    source_root: &Path,
    relative_path: &Path,
    content_hash: &str,
    cancel: &AtomicBool,
    post_decode_hook: impl FnOnce(),
) -> Result<Option<PreparedFeatureStage>, ReadinessStageError> {
    checkpoint(cancel, "feature analysis cancelled")?;
    let absolute_path = source_root.join(relative_path);
    if !file_content_hash_matches(&absolute_path, content_hash, cancel)? {
        return Ok(None);
    }
    let decoded = analysis_decode::decode_for_readiness(&absolute_path)?;
    post_decode_hook();
    checkpoint(cancel, "feature analysis cancelled before computation")?;
    let features = wavecrate_analysis::compute_feature_vector_v1_for_decoded_audio(&decoded)?;
    checkpoint(cancel, "feature analysis cancelled after computation")?;
    let feature_blob = wavecrate_analysis::vector::encode_f32_le_blob(&features);
    let light_dsp_blob = wavecrate_analysis::light_dsp_from_features_v1(&features)
        .map(|values| wavecrate_analysis::vector::encode_f32_le_blob(&values));
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    if !file_content_hash_matches(&absolute_path, content_hash, cancel)? {
        return Ok(None);
    }
    Ok(Some(PreparedFeatureStage {
        feature_version: wavecrate_analysis::vector::FEATURE_VERSION_V1,
        feature_blob,
        light_dsp_blob,
        rms,
        computed_at: now_epoch_seconds(),
        duration_seconds: decoded.duration_seconds,
        sample_rate_used: decoded.sample_rate_used,
    }))
}

/// Publish a prepared feature payload after revalidating the authoritative manifest.
#[allow(clippy::too_many_arguments)]
pub fn publish_feature_stage(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    prepared: &PreparedFeatureStage,
) -> Result<bool, String> {
    let sample_id = db::build_sample_id(source_id, relative_path);
    let relative = relative_path.to_string_lossy().replace('\\', "/");
    let manifest = conn
        .query_row(
            "SELECT file_size, modified_ns FROM wav_files
             WHERE path = ?1 AND content_hash = ?2 AND missing = 0",
            rusqlite::params![relative, content_hash],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|error| format!("Failed to read readiness sample manifest: {error}"))?;
    let Some((size, modified_ns)) = manifest else {
        return Ok(false);
    };
    let size = u64::try_from(size)
        .map_err(|_| format!("Readiness sample has a negative file size: {relative}"))?;
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_feature_publish")
        .map_err(|error| format!("Failed to start readiness feature transaction: {error}"))?;
    if !manifest_content_hash_is_current(&tx, relative_path, content_hash)? {
        db::telemetry::commit_transaction(tx, "readiness_feature_publish_stale")
            .map_err(|error| format!("Failed to commit stale feature skip: {error}"))?;
        return Ok(false);
    }
    db::upsert_samples_in_tx(
        &tx,
        &[db::SampleMetadata {
            sample_id: sample_id.clone(),
            content_hash: content_hash.to_string(),
            size,
            mtime_ns: modified_ns,
        }],
    )?;
    db::update_analysis_metadata(
        &tx,
        db::AnalysisMetadataUpdate {
            sample_id: &sample_id,
            content_hash: Some(content_hash),
            duration_seconds: prepared.duration_seconds,
            sr_used: prepared.sample_rate_used,
            analysis_version,
        },
    )?;
    db::upsert_analysis_features(
        &tx,
        &sample_id,
        &prepared.feature_blob,
        prepared.light_dsp_blob.as_deref(),
        prepared.rms,
        prepared.feature_version,
        prepared.computed_at,
    )?;
    db::upsert_cached_features(
        &tx,
        db::CachedFeaturesUpsert {
            content_hash,
            analysis_version,
            feat_version: prepared.feature_version,
            vec_blob: &prepared.feature_blob,
            light_dsp_blob: prepared.light_dsp_blob.as_deref(),
            rms: prepared.rms,
            computed_at: prepared.computed_at,
            duration_seconds: prepared.duration_seconds,
            sr_used: prepared.sample_rate_used,
        },
    )?;
    db::telemetry::commit_transaction(tx, "readiness_feature_publish")
        .map_err(|error| format!("Failed to commit readiness features: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
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
    let Some(input) = embedding_stage_input(
        conn,
        source_id,
        relative_path,
        content_hash,
        analysis_version,
    )?
    else {
        return Ok(false);
    };
    let _ = input;
    let sample_id = db::build_sample_id(source_id, relative_path);
    backfill::run_readiness_embedding_backfill(
        conn,
        source_root,
        &[sample_id],
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

/// Load the current feature vector needed by the embedding CPU phase.
pub fn embedding_stage_input(
    conn: &rusqlite::Connection,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
) -> Result<Option<EmbeddingStageInput>, String> {
    let sample_id = db::build_sample_id(source_id, relative_path);
    if db::sample_content_hash(conn, &sample_id)?.as_deref() != Some(content_hash) {
        return Ok(None);
    }
    let Some(cached) = db::cached_features_by_hash(
        conn,
        content_hash,
        analysis_version,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
    )?
    else {
        return Ok(None);
    };
    let features = wavecrate_analysis::decode_f32_le_blob(&cached.vec_blob)?;
    if features.len() != wavecrate_analysis::vector::FEATURE_VECTOR_LEN_V1 {
        return Ok(None);
    }
    Ok(Some(EmbeddingStageInput {
        features,
        computed_at: cached.computed_at,
    }))
}

/// Compute embedding and aspect payloads without a database connection.
pub fn prepare_embedding_stage(
    input: EmbeddingStageInput,
) -> Result<PreparedEmbeddingStage, String> {
    let embedding = wavecrate_analysis::similarity::embedding_from_features(&input.features)?;
    let aspects =
        wavecrate_analysis::aspects::aspect_descriptors_from_features_v1(&input.features)?;
    Ok(PreparedEmbeddingStage {
        embedding,
        aspect_blob: wavecrate_analysis::vector::encode_f32_le_blob(aspects.packed()),
        aspect_valid_mask: aspects.valid_mask(),
        computed_at: input.computed_at,
    })
}

/// Publish prepared embedding and aspect payloads under one fenced transaction.
#[allow(clippy::too_many_arguments)]
pub fn publish_embedding_stage(
    conn: &mut rusqlite::Connection,
    source_root: &Path,
    source_id: &str,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    prepared: &PreparedEmbeddingStage,
) -> Result<bool, String> {
    let sample_id = db::build_sample_id(source_id, relative_path);
    let tx = db::telemetry::begin_immediate_transaction(conn, "readiness_embedding_publish")
        .map_err(|error| format!("Failed to start readiness embedding transaction: {error}"))?;
    if db::sample_content_hash(&tx, &sample_id)?.as_deref() != Some(content_hash)
        || db::cached_features_by_hash(
            &tx,
            content_hash,
            analysis_version,
            wavecrate_analysis::vector::FEATURE_VERSION_V1,
        )?
        .is_none()
    {
        db::telemetry::commit_transaction(tx, "readiness_embedding_publish_stale")
            .map_err(|error| format!("Failed to commit stale embedding skip: {error}"))?;
        return Ok(false);
    }
    let embedding_blob = wavecrate_analysis::vector::encode_f32_le_blob(&prepared.embedding);
    db::upsert_embedding(
        &tx,
        db::EmbeddingUpsert {
            sample_id: &sample_id,
            model_id: wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            dim: wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            dtype: wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &embedding_blob,
            created_at: prepared.computed_at,
        },
    )?;
    db::upsert_cached_embedding(
        &tx,
        db::CachedEmbeddingUpsert {
            content_hash,
            analysis_version,
            model_id: wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            dim: wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            dtype: wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &embedding_blob,
            created_at: prepared.computed_at,
        },
    )?;
    db::upsert_aspect_descriptors(
        &tx,
        db::AspectDescriptorUpsert {
            sample_id: &sample_id,
            model_id: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            dim: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            dtype: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            l2_normed: true,
            valid_mask: prepared.aspect_valid_mask,
            vec_blob: &prepared.aspect_blob,
            created_at: prepared.computed_at,
        },
    )?;
    db::upsert_cached_aspect_descriptors(
        &tx,
        db::CachedAspectDescriptorsUpsert {
            content_hash,
            analysis_version,
            model_id: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
            dim: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DIM as i64,
            dtype: wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_DTYPE_F32,
            l2_normed: true,
            valid_mask: prepared.aspect_valid_mask,
            vec_blob: &prepared.aspect_blob,
            created_at: prepared.computed_at,
        },
    )?;
    db::telemetry::commit_transaction(tx, "readiness_embedding_publish")
        .map_err(|error| format!("Failed to commit readiness embedding: {error}"))?;
    crate::sample_sources::SourceDatabase::maybe_checkpoint_wal(
        source_root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    );
    Ok(true)
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

    #[test]
    fn compute_only_preparation_remains_available_while_database_writer_is_held() {
        let directory = tempfile::tempdir().expect("temporary readiness source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("compute-only-boundary"),
            directory.path().to_path_buf(),
        );
        let relative_path = Path::new("sample.wav");
        let absolute_path = source.root.join(relative_path);
        write_wav(&absolute_path, 440);
        let content_hash = blake3::hash(&std::fs::read(&absolute_path).expect("read wav"))
            .to_hex()
            .to_string();
        let mut connection = SourceDatabase::open_connection_for_background_job(&source.root)
            .expect("open source db");
        let writer = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .expect("hold source writer transaction");

        let prepared = prepare_feature_stage(
            &source.root,
            relative_path,
            &content_hash,
            &AtomicBool::new(false),
        )
        .expect("prepare feature payload")
        .expect("stable file produces feature payload");
        let embedding = prepare_embedding_stage(EmbeddingStageInput {
            features: wavecrate_analysis::decode_f32_le_blob(&prepared.feature_blob)
                .expect("decode prepared feature vector"),
            computed_at: prepared.computed_at,
        })
        .expect("prepare embedding payload");

        assert_eq!(
            embedding.embedding.len(),
            wavecrate_analysis::similarity::SIMILARITY_DIM
        );
        writer
            .rollback()
            .expect("release source writer transaction");
    }

    #[test]
    fn embedding_publication_rejects_replaced_content_generation() {
        let directory = tempfile::tempdir().expect("temporary readiness source");
        let source = SampleSource::new_with_id(
            SourceId::from_string("embedding-publication-fence"),
            directory.path().to_path_buf(),
        );
        let mut connection = SourceDatabase::open_connection_for_background_job(&source.root)
            .expect("open source db");
        let sample_id = db::build_sample_id(source.id.as_str(), Path::new("sample.wav"));
        db::upsert_samples(
            &mut connection,
            &[db::SampleMetadata {
                sample_id: sample_id.clone(),
                content_hash: "old-content".to_string(),
                size: 4,
                mtime_ns: 1,
            }],
        )
        .expect("seed sample row");
        let features = vec![0.25_f32; wavecrate_analysis::vector::FEATURE_VECTOR_LEN_V1];
        let feature_blob = wavecrate_analysis::vector::encode_f32_le_blob(&features);
        db::upsert_cached_features(
            &connection,
            db::CachedFeaturesUpsert {
                content_hash: "old-content",
                analysis_version: wavecrate_analysis::analysis_version(),
                feat_version: wavecrate_analysis::vector::FEATURE_VERSION_V1,
                vec_blob: &feature_blob,
                light_dsp_blob: None,
                rms: None,
                computed_at: 1,
                duration_seconds: 1.0,
                sr_used: wavecrate_analysis::ANALYSIS_SAMPLE_RATE,
            },
        )
        .expect("seed feature cache");
        let input = embedding_stage_input(
            &connection,
            source.id.as_str(),
            Path::new("sample.wav"),
            "old-content",
            wavecrate_analysis::analysis_version(),
        )
        .expect("load embedding input")
        .expect("feature input is current");
        let prepared = prepare_embedding_stage(input).expect("prepare embedding payload");
        connection
            .execute(
                "UPDATE samples SET content_hash = 'new-content' WHERE sample_id = ?1",
                [&sample_id],
            )
            .expect("replace sample content generation");

        assert!(
            !publish_embedding_stage(
                &mut connection,
                &source.root,
                source.id.as_str(),
                Path::new("sample.wav"),
                "old-content",
                wavecrate_analysis::analysis_version(),
                &prepared,
            )
            .expect("reject stale embedding publication")
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
