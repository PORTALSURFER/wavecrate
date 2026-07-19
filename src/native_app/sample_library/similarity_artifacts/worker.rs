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
    readiness::{
        ReadinessScopeKind, ReadinessSimilarityManifestRequest, ReadinessSimilarityPayloadContract,
        ReadinessStage, ReadinessStore, ReadinessTarget, ReadinessView,
    },
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
        let manifest_generation = connection
            .query_row(
                "SELECT COALESCE((SELECT CAST(value AS INTEGER) FROM metadata WHERE key = ?1), 0)",
                [META_WAV_PATHS_REVISION],
                |row| row.get(0),
            )
            .map_err(|error| format!("Read manifest revision failed: {error}"))?;
        ReadinessView::new(connection)
            .similarity_publication_is_current(
                &self.source_id,
                self.source_generation,
                &self.artifact_version,
                &self.membership_generation,
                manifest_generation,
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
    let pending_content_identities = ReadinessStore::new(&mut connection)
        .has_pending_file_content(source_id)
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
    let contract = ReadinessSimilarityPayloadContract::new(
        wavecrate_analysis::analysis_version(),
        SIMILARITY_MODEL_ID,
        wavecrate_analysis::vector::FEATURE_VERSION_V1,
        wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
        wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
        ASPECT_DESCRIPTOR_MODEL_ID,
        ASPECT_DESCRIPTOR_DIM as i64,
        ASPECT_DESCRIPTOR_DTYPE_F32,
    );
    let selection = ReadinessStore::new(connection)
        .similarity_manifest(ReadinessSimilarityManifestRequest::new(
            source_id,
            source_generation,
            contract,
        ))
        .map_err(|error| format!("Query exact similarity manifest failed: {error}"))?;
    let mut membership = blake3::Hasher::new();
    let mut manifest = Vec::new();
    let mut valid_identities = HashSet::new();
    for row in selection.rows {
        membership.update(row.scope_id.as_bytes());
        membership.update(&[0]);
        membership.update(row.content_generation.as_bytes());
        membership.update(&[0xff]);
        valid_identities.insert(row.scope_id);
        let embedding = analysis::decode_f32_le_blob(&row.embedding)?;
        if embedding.len() != usize::try_from(row.embedding_dim).unwrap_or_default()
            || embedding.len() != wavecrate_analysis::similarity::SIMILARITY_DIM
        {
            return Err(format!(
                "Invalid exact similarity embedding for {}",
                row.relative_path
            ));
        }
        manifest.push(analysis::ExactSimilarityManifestEntry {
            sample_id: format!("{source_id}::{}", row.relative_path.replace('\\', "/")),
            embedding,
        });
    }
    if manifest.len() != selection.target_count {
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
    let artifact_targets = ReadinessStore::new(connection)
        .embedding_artifact_targets(source_id, source_generation)
        .map_err(|error| format!("Query incomplete similarity artifacts failed: {error}"))?;
    for artifact in artifact_targets {
        if valid_identities.contains(&artifact.scope_id) {
            continue;
        }
        let target = ReadinessTarget::file(
            source_id,
            artifact.scope_id,
            artifact.relative_path,
            ReadinessStage::EmbeddingAspects,
            artifact.required_version,
            artifact.source_generation,
            artifact.content_generation,
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
    let mut conn = open_source_db(source)?;
    ReadinessStore::new(&mut conn)
        .reset_interrupted_work()
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
