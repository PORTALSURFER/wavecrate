#![cfg_attr(test, allow(dead_code))]

use std::{
    collections::HashSet,
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(not(test))]
use std::{
    io::Read,
    process::{Command, Stdio},
};

use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    db::META_WAV_PATHS_REVISION,
    readiness::{ReadinessScopeKind, ReadinessStage, ReadinessStore, ReadinessTarget},
};
use wavecrate_analysis::{
    self as analysis,
    aspects::{ASPECT_DESCRIPTOR_DIM, ASPECT_DESCRIPTOR_DTYPE_F32, ASPECT_DESCRIPTOR_MODEL_ID},
    similarity::SIMILARITY_MODEL_ID,
};

pub(in crate::native_app) use wavecrate::sample_sources::STARMAP_LAYOUT_UMAP_VERSION as NATIVE_SIMILARITY_UMAP_VERSION;
const NATIVE_SIMILARITY_CLUSTER_MIN_SIZE: usize = 10;
const NATIVE_SIMILARITY_CLUSTER_VERSION: &str = "hdbscan-layout-v1-min10";
const INTERNAL_SIMILARITY_FINALIZER_ARG: &str = "--wavecrate-internal-similarity-finalizer-v1";

pub(in crate::native_app) fn native_similarity_artifact_version() -> String {
    format!(
        "similarity-bundle-v1|layout={}|cluster={}|ann={}|analysis={}|embedding={}|aspects={}",
        NATIVE_SIMILARITY_UMAP_VERSION,
        NATIVE_SIMILARITY_CLUSTER_VERSION,
        analysis::ann_index::contract_version(),
        wavecrate_analysis::analysis_version(),
        SIMILARITY_MODEL_ID,
        ASPECT_DESCRIPTOR_MODEL_ID,
    )
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(in crate::native_app) struct SimilarityPublicationFence {
    source_id: String,
    source_generation: i64,
    membership_generation: String,
    artifact_version: String,
}

impl SimilarityPublicationFence {
    pub(in crate::native_app) fn for_readiness_target(
        target: &ReadinessTarget,
    ) -> Result<Self, String> {
        if target.scope_kind != ReadinessScopeKind::Source
            || target.stage != ReadinessStage::SimilarityLayout
        {
            return Err("similarity publication requires a source-level layout target".to_string());
        }
        Ok(Self {
            source_id: target.source_id.clone(),
            source_generation: target.source_generation,
            membership_generation: target.content_generation.clone(),
            artifact_version: target.required_version.clone(),
        })
    }

    fn is_current(&self, connection: &rusqlite::Connection) -> Result<bool, String> {
        connection
            .query_row(
                "SELECT EXISTS(
                    SELECT 1
                    FROM source_readiness_sources AS source
                    JOIN source_readiness_targets AS target
                      ON target.source_id = source.source_id
                    WHERE source.source_id = ?1
                      AND source.source_generation = ?2
                      AND source.availability = 'active'
                      AND target.scope_kind = 'source'
                      AND target.scope_id = ?1
                      AND target.stage = 'similarity_layout'
                      AND target.required_version = ?3
                      AND target.source_generation = ?2
                      AND target.content_generation = ?4
                      AND target.eligibility = 'eligible'
                      AND COALESCE(
                          (SELECT CAST(value AS INTEGER) FROM metadata WHERE key = ?5),
                          0
                      ) = ?2
                )",
                rusqlite::params![
                    self.source_id,
                    self.source_generation,
                    self.artifact_version,
                    self.membership_generation,
                    META_WAV_PATHS_REVISION,
                ],
                |row| row.get(0),
            )
            .map_err(|error| format!("Validate similarity readiness generation failed: {error}"))
    }
}

pub(in crate::native_app) fn finalize_similarity_artifacts_if_ready(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    #[cfg(test)]
    {
        return finalize_if_ready(source, publication_fence, cancel);
    }
    #[cfg(not(test))]
    {
        finalize_similarity_artifacts_in_child(source, publication_fence, cancel)
    }
}

pub(in crate::native_app) fn run_internal_similarity_finalizer_from_args()
-> Result<Option<bool>, String> {
    let mut args = std::env::args();
    let _executable = args.next();
    if args.next().as_deref() != Some(INTERNAL_SIMILARITY_FINALIZER_ARG) {
        return Ok(None);
    }
    let source_json = args
        .next()
        .ok_or_else(|| "Internal similarity finalizer is missing its source".to_string())?;
    let fence_json = args.next().ok_or_else(|| {
        "Internal similarity finalizer is missing its publication fence".to_string()
    })?;
    if args.next().is_some() {
        return Err("Internal similarity finalizer received unexpected arguments".to_string());
    }
    let source = serde_json::from_str::<SampleSource>(&source_json)
        .map_err(|error| format!("Decode internal similarity source failed: {error}"))?;
    let publication_fence = serde_json::from_str::<SimilarityPublicationFence>(&fence_json)
        .map_err(|error| format!("Decode internal similarity fence failed: {error}"))?;
    let cancel = AtomicBool::new(false);
    finalize_if_ready(&source, &publication_fence, &cancel).map(Some)
}

#[cfg(not(test))]
fn finalize_similarity_artifacts_in_child(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("Resolve similarity finalizer executable failed: {error}"))?;
    let source_json = serde_json::to_string(source)
        .map_err(|error| format!("Encode internal similarity source failed: {error}"))?;
    let fence_json = serde_json::to_string(publication_fence)
        .map_err(|error| format!("Encode internal similarity fence failed: {error}"))?;
    let child = Command::new(executable)
        .arg(INTERNAL_SIMILARITY_FINALIZER_ARG)
        .arg(source_json)
        .arg(fence_json)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Start similarity finalizer process failed: {error}"))?;
    let Some(mut child) = crate::native_app::source_processing::wait_for_cancellable_child(
        child,
        cancel,
        "similarity finalizer",
    )?
    else {
        return Ok(false);
    };
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout)
            .map_err(|error| format!("Read similarity finalizer result failed: {error}"))?;
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr)
            .map_err(|error| format!("Read similarity finalizer error failed: {error}"))?;
    }
    let status = child
        .wait()
        .map_err(|error| format!("Join similarity finalizer process failed: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Similarity finalizer process failed with {status}: {}",
            stderr.trim()
        ));
    }
    serde_json::from_str::<bool>(stdout.trim())
        .map_err(|error| format!("Decode similarity finalizer result failed: {error}"))
}

fn finalize_if_ready(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    finalize_exact_readiness_similarity(source, publication_fence, cancel)
}

fn finalize_exact_readiness_similarity(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    let SimilarityPublicationFence {
        source_id,
        source_generation,
        membership_generation,
        artifact_version,
    } = publication_fence;
    if source_id != source.id.as_str() || artifact_version != &native_similarity_artifact_version()
    {
        return Err(
            "Similarity finalizer fence does not match the active source contract".to_string(),
        );
    }
    let mut connection = open_source_db(source)?;
    if !publication_fence.is_current(&connection)? {
        return Ok(false);
    }
    let pending_content_identities: bool = connection
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM source_readiness_targets
                WHERE source_id = ?1
                  AND scope_kind = 'file'
                  AND stage = 'indexed_identity'
                  AND content_generation LIKE 'pending-%'
             )",
            [source_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Check deferred similarity identities failed: {error}"))?;
    if pending_content_identities {
        return Ok(false);
    }
    let Some(manifest) = exact_similarity_manifest(
        &mut connection,
        source_id,
        *source_generation,
        membership_generation,
    )?
    else {
        return Ok(false);
    };
    let source_sample_id_prefix = format!("{source_id}::");
    let artifact_generation = blake3::hash(
        format!("{membership_generation}|{source_generation}|{artifact_version}").as_bytes(),
    )
    .to_hex()
    .to_string();
    let publication = analysis::rebuild_exact_similarity_artifacts(
        &mut connection,
        analysis::ExactSimilarityArtifactRequest {
            source_id,
            source_sample_id_prefix: &source_sample_id_prefix,
            model_id: SIMILARITY_MODEL_ID,
            layout_version: NATIVE_SIMILARITY_UMAP_VERSION,
            artifact_contract_version: artifact_version,
            artifact_generation: &artifact_generation,
            cluster_config: analysis::hdbscan::HdbscanConfig {
                min_cluster_size: NATIVE_SIMILARITY_CLUSTER_MIN_SIZE,
                min_samples: None,
                allow_single_cluster: false,
            },
        },
        &manifest,
        cancel,
        &|connection| publication_fence.is_current(connection),
    )?;
    if publication.is_none() || cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    Ok(true)
}

fn exact_similarity_manifest(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    source_generation: i64,
    expected_membership_generation: &str,
) -> Result<Option<Vec<analysis::ExactSimilarityManifestEntry>>, String> {
    let target_count: i64 = connection
        .query_row(
            "SELECT COUNT(*)
             FROM source_readiness_targets
             WHERE source_id = ?1
               AND scope_kind = 'file'
               AND stage = 'embedding_aspects'
               AND eligibility = 'eligible'",
            [source_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("Count exact similarity targets failed: {error}"))?;
    let mut statement = connection
        .prepare(
            "SELECT target.scope_id, target.relative_path, target.content_generation,
                    embedding.dim, embedding.vec
             FROM source_readiness_targets AS target
             JOIN source_readiness_artifacts AS artifact
               ON artifact.source_id = target.source_id
              AND artifact.scope_kind = target.scope_kind
              AND artifact.scope_id = target.scope_id
              AND artifact.stage = target.stage
              AND artifact.artifact_version = target.required_version
              AND artifact.content_generation = target.content_generation
             JOIN samples AS sample
               ON sample.sample_id = target.source_id || '::' || target.relative_path
              AND sample.content_hash = target.content_generation
              AND sample.analysis_version = ?4
             JOIN features AS feature
               ON feature.sample_id = sample.sample_id
             JOIN analysis_cache_features AS cached_feature
               ON cached_feature.content_hash = target.content_generation
              AND cached_feature.analysis_version = ?4
              AND cached_feature.feat_version = feature.feat_version
              AND cached_feature.vec_blob = feature.vec_blob
              AND cached_feature.light_dsp_blob IS feature.light_dsp_blob
              AND cached_feature.rms IS feature.rms
             JOIN embeddings AS embedding
               ON embedding.sample_id = sample.sample_id
              AND embedding.model_id = ?3
             JOIN analysis_cache_embeddings AS cached_embedding
               ON cached_embedding.content_hash = target.content_generation
              AND cached_embedding.analysis_version = ?4
              AND cached_embedding.model_id = embedding.model_id
              AND cached_embedding.dim = embedding.dim
              AND cached_embedding.dtype = embedding.dtype
              AND cached_embedding.l2_normed = embedding.l2_normed
              AND cached_embedding.vec = embedding.vec
             JOIN similarity_aspect_descriptors AS aspects
               ON aspects.sample_id = sample.sample_id
              AND aspects.model_id = ?5
             JOIN analysis_cache_aspect_descriptors AS cached_aspects
               ON cached_aspects.content_hash = target.content_generation
              AND cached_aspects.analysis_version = ?4
              AND cached_aspects.model_id = aspects.model_id
              AND cached_aspects.dim = aspects.dim
              AND cached_aspects.dtype = aspects.dtype
              AND cached_aspects.l2_normed = aspects.l2_normed
              AND cached_aspects.valid_mask = aspects.valid_mask
              AND cached_aspects.vec = aspects.vec
             WHERE target.source_id = ?1
               AND target.scope_kind = 'file'
               AND target.stage = 'embedding_aspects'
              AND target.source_generation = ?2
              AND target.eligibility = 'eligible'
              AND feature.feat_version = ?6
              AND embedding.dim = ?7
              AND embedding.dtype = ?8
              AND embedding.l2_normed = 1
              AND aspects.dim = ?9
              AND aspects.dtype = ?10
              AND aspects.l2_normed = 1
             ORDER BY target.relative_path",
        )
        .map_err(|error| format!("Prepare exact similarity manifest failed: {error}"))?;
    let rows = statement
        .query_map(
            rusqlite::params![
                source_id,
                source_generation,
                SIMILARITY_MODEL_ID,
                wavecrate_analysis::analysis_version(),
                ASPECT_DESCRIPTOR_MODEL_ID,
                wavecrate_analysis::vector::FEATURE_VERSION_V1,
                wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
                wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
                ASPECT_DESCRIPTOR_DIM as i64,
                ASPECT_DESCRIPTOR_DTYPE_F32,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Vec<u8>>(4)?,
                ))
            },
        )
        .map_err(|error| format!("Query exact similarity manifest failed: {error}"))?;
    let mut membership = blake3::Hasher::new();
    let mut manifest = Vec::new();
    let mut valid_identities = HashSet::new();
    for row in rows {
        let (identity, relative_path, content_generation, dim, vector) =
            row.map_err(|error| format!("Decode exact similarity manifest failed: {error}"))?;
        membership.update(identity.as_bytes());
        membership.update(&[0]);
        membership.update(content_generation.as_bytes());
        membership.update(&[0xff]);
        valid_identities.insert(identity);
        let embedding = analysis::decode_f32_le_blob(&vector)?;
        if embedding.len() != usize::try_from(dim).unwrap_or_default()
            || embedding.len() != wavecrate_analysis::similarity::SIMILARITY_DIM
        {
            return Err(format!(
                "Invalid exact similarity embedding for {relative_path}"
            ));
        }
        manifest.push(analysis::ExactSimilarityManifestEntry {
            sample_id: format!("{source_id}::{}", relative_path.replace('\\', "/")),
            embedding,
        });
    }
    drop(statement);
    if i64::try_from(manifest.len()).unwrap_or(i64::MAX) != target_count {
        invalidate_incomplete_embedding_artifacts(
            connection,
            source_id,
            source_generation,
            &valid_identities,
        )?;
        return Ok(None);
    }
    let actual_membership_generation = membership.finalize().to_hex().to_string();
    if actual_membership_generation != expected_membership_generation {
        return Err("Similarity manifest generation changed before finalization".to_string());
    }
    Ok(Some(manifest))
}

fn invalidate_incomplete_embedding_artifacts(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    source_generation: i64,
    valid_identities: &HashSet<String>,
) -> Result<(), String> {
    let artifact_targets = {
        let mut statement = connection
            .prepare(
                "SELECT target.scope_id, target.relative_path, target.required_version,
                        target.source_generation, target.content_generation
                 FROM source_readiness_targets AS target
                 JOIN source_readiness_artifacts AS artifact
                   ON artifact.source_id = target.source_id
                  AND artifact.scope_kind = target.scope_kind
                  AND artifact.scope_id = target.scope_id
                  AND artifact.stage = target.stage
                  AND artifact.artifact_version = target.required_version
                  AND artifact.content_generation = target.content_generation
                 WHERE target.source_id = ?1
                   AND target.scope_kind = 'file'
                   AND target.stage = 'embedding_aspects'
                   AND target.source_generation = ?2
                   AND target.eligibility = 'eligible'",
            )
            .map_err(|error| format!("Prepare incomplete similarity artifacts failed: {error}"))?;
        statement
            .query_map(rusqlite::params![source_id, source_generation], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|error| format!("Query incomplete similarity artifacts failed: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Decode incomplete similarity artifacts failed: {error}"))?
    };
    for (identity, relative_path, version, generation, content_generation) in artifact_targets {
        if valid_identities.contains(&identity) {
            continue;
        }
        let target = ReadinessTarget::file(
            source_id,
            identity,
            relative_path,
            ReadinessStage::EmbeddingAspects,
            version,
            generation,
            content_generation,
        );
        ReadinessStore::new(connection)
            .invalidate_artifact(&target)
            .map_err(|error| {
                format!("Invalidate incomplete similarity artifact failed: {error}")
            })?;
    }
    Ok(())
}

pub(in crate::native_app) fn reset_interrupted_readiness_jobs(
    source: &SampleSource,
) -> Result<usize, String> {
    let conn = open_source_db(source)?;
    conn.execute(
        "UPDATE analysis_jobs
         SET status = 'pending',
             running_at = NULL,
             lease_expires_at = NULL
         WHERE status = 'running'
           AND readiness_managed = 1",
        [],
    )
    .map_err(|error| format!("Reset interrupted readiness jobs failed: {error}"))
}

pub(super) fn open_source_db(source: &SampleSource) -> Result<rusqlite::Connection, String> {
    let database_root = source
        .database_root()
        .map_err(|err| format!("Resolve source metadata location failed: {err}"))?;
    SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))
}
