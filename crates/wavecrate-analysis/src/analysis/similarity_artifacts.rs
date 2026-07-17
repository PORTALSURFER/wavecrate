//! Exact source-manifest similarity artifact publication.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{Connection, params};

use super::hdbscan::{HdbscanConfig, compute_layout_clusters};
use super::{ann_index, umap};

const EXACT_MANIFEST_TABLE: &str = "temp_exact_similarity_manifest";

/// One exact current source-manifest identity and its versioned similarity vector.
#[derive(Clone, Debug, PartialEq)]
pub struct ExactSimilarityManifestEntry {
    /// Materialized sample identifier owned by the source-relative path.
    pub sample_id: String,
    /// Current embedding for the sample's committed content generation.
    pub embedding: Vec<f32>,
}

/// Versioned contract and source ownership for one exact similarity publication.
#[derive(Clone, Copy, Debug)]
pub struct ExactSimilarityArtifactRequest<'a> {
    /// Configured source identity that owns the readiness targets and work rows.
    pub source_id: &'a str,
    /// Prefix that owns every materialized sample ID in the source database.
    pub source_sample_id_prefix: &'a str,
    /// Embedding model represented by the exact manifest vectors.
    pub model_id: &'a str,
    /// Persisted two-dimensional layout schema and algorithm version.
    pub layout_version: &'a str,
    /// Full feature, embedding, descriptor, ANN, layout, and cluster contract version.
    pub artifact_contract_version: &'a str,
    /// Exact content-membership and source generation being published.
    pub artifact_generation: &'a str,
    /// Cluster algorithm parameters for the same exact layout generation.
    pub cluster_config: HdbscanConfig,
}

/// Counts from one atomically published source similarity generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactSimilarityPublication {
    /// Exact eligible identities included in ANN, layout, and cluster artifacts.
    pub identities: usize,
    /// Obsolete path-derived artifact rows removed during publication.
    pub pruned_rows: usize,
}

/// Rebuild ANN, layout, and clusters from one exact eligible source manifest.
///
/// All SQL rows and the generation-specific ANN metadata pointer are committed in one immediate
/// transaction after the caller's generation fence succeeds. Content-addressed cache tables are
/// deliberately retained while obsolete path/sample-derived rows are pruned.
pub fn rebuild_exact_similarity_artifacts(
    connection: &mut Connection,
    request: ExactSimilarityArtifactRequest<'_>,
    manifest: &[ExactSimilarityManifestEntry],
    cancel: &AtomicBool,
    publication_fence: &impl Fn(&Connection) -> Result<bool, String>,
) -> Result<Option<ExactSimilarityPublication>, String> {
    validate_manifest(
        request.source_id,
        request.source_sample_id_prefix,
        request.artifact_generation,
        manifest,
    )?;
    if cancel.load(Ordering::Acquire) {
        return Ok(None);
    }
    let embeddings = manifest
        .iter()
        .map(|entry| (entry.sample_id.clone(), entry.embedding.clone()))
        .collect::<Vec<_>>();
    let sample_ids = embeddings
        .iter()
        .map(|(sample_id, _)| sample_id.clone())
        .collect::<Vec<_>>();
    let layout = umap::compute_layout_for_embeddings(&embeddings, 0, 0.95)?;
    if cancel.load(Ordering::Acquire) {
        return Ok(None);
    }
    let labels = compute_layout_clusters(&sample_ids, &layout, request.cluster_config)?;
    if cancel.load(Ordering::Acquire) {
        return Ok(None);
    }
    prepare_exact_manifest_table(connection, &sample_ids)?;
    let mut pruned_rows = 0usize;
    let state = serde_json::json!({
        "state": "current",
        "artifact_generation": request.artifact_generation,
        "model_id": request.model_id,
        "layout_version": request.layout_version,
        "artifact_contract_version": request.artifact_contract_version,
        "identity_count": sample_ids.len(),
    })
    .to_string();
    let published = ann_index::publish_exact_index_with_transaction(
        connection,
        &embeddings,
        request.artifact_generation,
        publication_fence,
        |transaction| {
            pruned_rows = prune_path_derived_rows(
                transaction,
                request.source_id,
                request.source_sample_id_prefix,
            )?;
            replace_layout_rows(
                transaction,
                request.source_sample_id_prefix,
                request.model_id,
                request.layout_version,
                &sample_ids,
                &layout,
            )?;
            replace_cluster_rows(
                transaction,
                request.source_sample_id_prefix,
                request.model_id,
                request.layout_version,
                &sample_ids,
                &labels,
            )?;
            transaction
                .execute(
                    "INSERT INTO metadata (key, value)
                     VALUES ('similarity_artifact_state_v1', ?1)
                     ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                    [&state],
                )
                .map_err(|error| format!("Publish similarity generation state failed: {error}"))?;
            transaction
                .execute("DELETE FROM metadata WHERE key = 'ann_index_dirty_v1'", [])
                .map_err(|error| format!("Clear ANN dirty state failed: {error}"))?;
            Ok(())
        },
    )?;
    drop_exact_manifest_table(connection)?;
    Ok(published.then_some(ExactSimilarityPublication {
        identities: sample_ids.len(),
        pruned_rows,
    }))
}

fn validate_manifest(
    source_id: &str,
    source_sample_id_prefix: &str,
    artifact_generation: &str,
    manifest: &[ExactSimilarityManifestEntry],
) -> Result<(), String> {
    if source_id.trim().is_empty()
        || source_sample_id_prefix != format!("{source_id}::")
        || artifact_generation.trim().is_empty()
    {
        return Err(
            "Exact similarity publication requires non-empty source and generation identities"
                .to_string(),
        );
    }
    let mut identities = BTreeSet::new();
    for entry in manifest {
        if !entry.sample_id.starts_with(source_sample_id_prefix) {
            return Err(format!(
                "Similarity manifest sample does not belong to source: {}",
                entry.sample_id
            ));
        }
        if !identities.insert(entry.sample_id.as_str()) {
            return Err(format!(
                "Duplicate similarity manifest identity: {}",
                entry.sample_id
            ));
        }
    }
    Ok(())
}

fn prepare_exact_manifest_table(
    connection: &Connection,
    sample_ids: &[String],
) -> Result<(), String> {
    connection
        .execute_batch(&format!(
            "CREATE TEMP TABLE IF NOT EXISTS {EXACT_MANIFEST_TABLE} (
                sample_id TEXT PRIMARY KEY
             ) WITHOUT ROWID;
             DELETE FROM {EXACT_MANIFEST_TABLE};"
        ))
        .map_err(|error| format!("Prepare exact similarity manifest failed: {error}"))?;
    let mut insert = connection
        .prepare(&format!(
            "INSERT INTO {EXACT_MANIFEST_TABLE} (sample_id) VALUES (?1)"
        ))
        .map_err(|error| format!("Prepare exact similarity identity insert failed: {error}"))?;
    for sample_id in sample_ids {
        insert
            .execute([sample_id])
            .map_err(|error| format!("Insert exact similarity identity failed: {error}"))?;
    }
    Ok(())
}

fn drop_exact_manifest_table(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(&format!("DROP TABLE IF EXISTS {EXACT_MANIFEST_TABLE};"))
        .map_err(|error| format!("Drop exact similarity manifest failed: {error}"))
}

fn prune_path_derived_rows(
    transaction: &rusqlite::Transaction<'_>,
    source_id: &str,
    source_sample_id_prefix: &str,
) -> Result<usize, String> {
    let mut pruned = 0usize;
    pruned += transaction
        .execute(
            "DELETE FROM analysis_jobs AS job
             WHERE job.source_id = ?1
               AND job.readiness_managed = 1
               AND NOT EXISTS (
                   SELECT 1 FROM source_readiness_targets AS target
                   WHERE target.source_id = job.source_id
                     AND target.scope_kind = job.readiness_scope_kind
                     AND target.scope_id = job.readiness_scope_id
                     AND target.stage = job.readiness_stage
                     AND target.required_version = job.artifact_version
                     AND target.content_generation = job.content_generation
                     AND (
                         target.scope_kind = 'file'
                         OR target.source_generation = job.source_generation
                     )
               )",
            [source_id],
        )
        .map_err(|error| format!("Prune obsolete readiness jobs failed: {error}"))?;
    pruned += transaction
        .execute(
            "DELETE FROM analysis_jobs AS job
             WHERE job.source_id = ?1
               AND job.readiness_managed = 0
               AND NOT EXISTS (
                   SELECT 1 FROM source_readiness_targets AS target
                   WHERE target.source_id = job.source_id
                     AND target.scope_kind = 'file'
                     AND target.relative_path IS NOT NULL
                     AND target.source_id || '::' || target.relative_path = job.sample_id
               )",
            [source_id],
        )
        .map_err(|error| format!("Prune stale similarity jobs failed: {error}"))?;
    pruned += transaction
        .execute(
            "DELETE FROM source_readiness_artifacts AS artifact
             WHERE artifact.source_id = ?1
               AND NOT EXISTS (
                   SELECT 1 FROM source_readiness_targets AS target
                   WHERE target.source_id = artifact.source_id
                     AND target.scope_kind = artifact.scope_kind
                     AND target.scope_id = artifact.scope_id
                     AND target.stage = artifact.stage
                     AND target.required_version = artifact.artifact_version
                     AND target.content_generation = artifact.content_generation
                     AND (
                         target.scope_kind = 'file'
                         OR target.source_generation = artifact.source_generation
                     )
               )",
            [source_id],
        )
        .map_err(|error| format!("Prune obsolete readiness artifacts failed: {error}"))?;
    for table in [
        "analysis_features",
        "features",
        "embeddings",
        "similarity_aspect_descriptors",
        "layout_umap",
        "hdbscan_clusters",
        "samples",
    ] {
        let sql = format!(
            "DELETE FROM {table}
             WHERE substr(sample_id, 1, length(?1)) = ?1
               AND NOT EXISTS (
                   SELECT 1 FROM {EXACT_MANIFEST_TABLE} AS manifest
                   WHERE manifest.sample_id = {table}.sample_id
               )"
        );
        pruned += transaction
            .execute(&sql, [source_sample_id_prefix])
            .map_err(|error| format!("Prune stale {table} rows failed: {error}"))?;
    }
    Ok(pruned)
}

fn replace_layout_rows(
    transaction: &rusqlite::Transaction<'_>,
    source_sample_id_prefix: &str,
    model_id: &str,
    layout_version: &str,
    sample_ids: &[String],
    layout: &[[f32; 2]],
) -> Result<(), String> {
    if sample_ids.len() != layout.len() {
        return Err("Similarity layout identity count mismatch".to_string());
    }
    transaction
        .execute(
            "DELETE FROM layout_umap
             WHERE substr(sample_id, 1, length(?1)) = ?1",
            [source_sample_id_prefix],
        )
        .map_err(|error| format!("Clear previous source layout failed: {error}"))?;
    let mut insert = transaction
        .prepare(
            "INSERT INTO layout_umap
             (sample_id, model_id, umap_version, x, y, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, unixepoch())",
        )
        .map_err(|error| format!("Prepare exact layout insert failed: {error}"))?;
    for (sample_id, point) in sample_ids.iter().zip(layout) {
        insert
            .execute(params![
                sample_id,
                model_id,
                layout_version,
                point[0],
                point[1]
            ])
            .map_err(|error| format!("Insert exact layout row failed: {error}"))?;
    }
    Ok(())
}

fn replace_cluster_rows(
    transaction: &rusqlite::Transaction<'_>,
    source_sample_id_prefix: &str,
    model_id: &str,
    layout_version: &str,
    sample_ids: &[String],
    labels: &[i32],
) -> Result<(), String> {
    if sample_ids.len() != labels.len() {
        return Err("Similarity cluster identity count mismatch".to_string());
    }
    transaction
        .execute(
            "DELETE FROM hdbscan_clusters
             WHERE substr(sample_id, 1, length(?1)) = ?1",
            [source_sample_id_prefix],
        )
        .map_err(|error| format!("Clear previous source clusters failed: {error}"))?;
    let mut insert = transaction
        .prepare(
            "INSERT INTO hdbscan_clusters
             (sample_id, model_id, method, umap_version, cluster_id, created_at)
             VALUES (?1, ?2, 'umap', ?3, ?4, unixepoch())",
        )
        .map_err(|error| format!("Prepare exact cluster insert failed: {error}"))?;
    for (sample_id, label) in sample_ids.iter().zip(labels) {
        insert
            .execute(params![sample_id, model_id, layout_version, label])
            .map_err(|error| format!("Insert exact cluster row failed: {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{ann_index, similarity};

    #[test]
    fn exact_publication_prunes_stale_rows_and_replaces_changed_ann_vectors() {
        let directory = tempfile::tempdir().expect("similarity artifact directory");
        let database_path = directory.path().join("source.db");
        let mut connection = Connection::open(database_path).expect("open source database");
        create_schema(&connection);
        seed_path_artifacts(&connection, "source::current-a");
        seed_path_artifacts(&connection, "source::current-b");
        seed_path_artifacts(&connection, "source::deleted");
        seed_current_readiness(&connection, "source::current-a", "identity-a", true);
        seed_current_readiness(&connection, "source::current-b", "identity-b", false);
        seed_obsolete_readiness(&connection);

        let first = vec![
            manifest_entry("source::current-a", axis_embedding(0, 1.0)),
            manifest_entry("source::current-b", axis_embedding(1, 1.0)),
        ];
        let published = rebuild_exact_similarity_artifacts(
            &mut connection,
            request("generation-one"),
            &first,
            &AtomicBool::new(false),
            &|_| Ok(true),
        )
        .expect("publish first generation")
        .expect("first generation accepted");

        assert_eq!(published.identities, 2);
        assert!(published.pruned_rows >= 6);
        assert_exact_membership(&connection, "layout_umap", 2);
        assert_exact_membership(&connection, "hdbscan_clusters", 2);
        assert_eq!(ann_meta_count(&connection), 2);
        assert_eq!(table_count(&connection, "analysis_jobs"), 3);
        assert_eq!(table_count(&connection, "source_readiness_artifacts"), 1);
        assert_eq!(
            nearest_id(&connection, &axis_embedding(0, 1.0)),
            "source::current-a"
        );

        let second = vec![
            manifest_entry("source::current-a", axis_embedding(0, -1.0)),
            manifest_entry("source::current-b", axis_embedding(1, 1.0)),
        ];
        rebuild_exact_similarity_artifacts(
            &mut connection,
            request("generation-two"),
            &second,
            &AtomicBool::new(false),
            &|_| Ok(true),
        )
        .expect("publish changed generation")
        .expect("changed generation accepted");

        assert_eq!(
            nearest_id(&connection, &axis_embedding(0, 1.0)),
            "source::current-b",
            "the rebuilt ANN must replace the vector for an existing sample id"
        );
        ann_index::evict_index_for_test(&connection).expect("simulate ANN process restart");
        assert_eq!(
            nearest_id(&connection, &axis_embedding(0, 1.0)),
            "source::current-b",
            "the exact ANN generation must reload after a process restart"
        );
        let state: String = connection
            .query_row(
                "SELECT value FROM metadata WHERE key = 'similarity_artifact_state_v1'",
                [],
                |row| row.get(0),
            )
            .expect("read similarity generation state");
        assert!(state.contains("generation-two"));
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM metadata WHERE key = 'ann_index_dirty_v1'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .expect("count ANN dirty state"),
            0
        );
    }

    #[test]
    fn stale_generation_rejects_the_whole_similarity_bundle() {
        let directory = tempfile::tempdir().expect("similarity artifact directory");
        let database_path = directory.path().join("source.db");
        let mut connection = Connection::open(database_path).expect("open source database");
        create_schema(&connection);
        seed_path_artifacts(&connection, "source::current");
        connection
            .execute(
                "INSERT INTO layout_umap VALUES
                 ('source::current', ?1, 'old', 4.0, 8.0, 1)",
                [similarity::SIMILARITY_MODEL_ID],
            )
            .expect("seed old layout");

        let result = rebuild_exact_similarity_artifacts(
            &mut connection,
            request("stale-generation"),
            &[manifest_entry("source::current", axis_embedding(0, 1.0))],
            &AtomicBool::new(false),
            &|_| Ok(false),
        )
        .expect("reject stale generation");

        assert_eq!(result, None);
        assert_eq!(
            connection
                .query_row(
                    "SELECT umap_version FROM layout_umap WHERE sample_id = 'source::current'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect("read retained layout"),
            "old"
        );
        assert_eq!(ann_meta_count(&connection), 0);
    }

    fn create_schema(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE ann_index_meta (
                    model_id TEXT PRIMARY KEY, index_path TEXT NOT NULL, count INTEGER NOT NULL,
                    params_json TEXT NOT NULL, updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE source_readiness_targets (
                    source_id TEXT NOT NULL, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
                    relative_path TEXT, stage TEXT NOT NULL, required_version TEXT NOT NULL,
                    source_generation INTEGER NOT NULL, content_generation TEXT NOT NULL
                 );
                 CREATE TABLE source_readiness_artifacts (
                    source_id TEXT NOT NULL, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
                    stage TEXT NOT NULL, artifact_version TEXT NOT NULL,
                    source_generation INTEGER NOT NULL, content_generation TEXT NOT NULL
                 );
                 CREATE TABLE analysis_jobs (
                    sample_id TEXT NOT NULL, source_id TEXT NOT NULL,
                    readiness_managed INTEGER NOT NULL, readiness_scope_kind TEXT,
                    readiness_scope_id TEXT, readiness_stage TEXT, artifact_version TEXT,
                    source_generation INTEGER, content_generation TEXT
                 );
                 CREATE TABLE analysis_features (sample_id TEXT PRIMARY KEY);
                 CREATE TABLE features (sample_id TEXT PRIMARY KEY);
                 CREATE TABLE embeddings (
                    sample_id TEXT PRIMARY KEY, model_id TEXT NOT NULL, dim INTEGER NOT NULL,
                    dtype TEXT NOT NULL, l2_normed INTEGER NOT NULL, vec BLOB NOT NULL,
                    created_at INTEGER NOT NULL
                 );
                 CREATE TABLE similarity_aspect_descriptors (sample_id TEXT PRIMARY KEY);
                 CREATE TABLE layout_umap (
                    sample_id TEXT PRIMARY KEY, model_id TEXT NOT NULL,
                    umap_version TEXT NOT NULL, x REAL NOT NULL, y REAL NOT NULL,
                    created_at INTEGER NOT NULL
                 );
                 CREATE TABLE hdbscan_clusters (
                    sample_id TEXT NOT NULL, model_id TEXT NOT NULL, method TEXT NOT NULL,
                    umap_version TEXT NOT NULL, cluster_id INTEGER NOT NULL,
                    created_at INTEGER NOT NULL,
                    PRIMARY KEY (sample_id, model_id, method, umap_version)
                 );
                 CREATE TABLE samples (sample_id TEXT PRIMARY KEY);",
            )
            .expect("create exact similarity schema");
    }

    fn seed_path_artifacts(connection: &Connection, sample_id: &str) {
        for table in [
            "analysis_features",
            "features",
            "similarity_aspect_descriptors",
            "samples",
        ] {
            connection
                .execute(
                    &format!("INSERT INTO {table} (sample_id) VALUES (?1)"),
                    [sample_id],
                )
                .expect("seed path-derived row");
        }
        connection
            .execute(
                "INSERT INTO analysis_jobs (sample_id, source_id, readiness_managed)
                 VALUES (?1, 'source', 0)",
                [sample_id],
            )
            .expect("seed path-derived job");
        connection
            .execute(
                "INSERT INTO embeddings VALUES (?1, ?2, ?3, 'f32', 1, ?4, 1)",
                params![
                    sample_id,
                    similarity::SIMILARITY_MODEL_ID,
                    similarity::SIMILARITY_DIM as i64,
                    crate::analysis::vector::encode_f32_le_blob(&axis_embedding(0, 1.0)),
                ],
            )
            .expect("seed path-derived embedding");
    }

    fn seed_current_readiness(
        connection: &Connection,
        sample_id: &str,
        identity: &str,
        with_artifact: bool,
    ) {
        let relative_path = sample_id
            .strip_prefix("source::")
            .expect("source sample id");
        connection
            .execute(
                "INSERT INTO source_readiness_targets VALUES
                 ('source', 'file', ?1, ?2, 'embedding_aspects', 'v1', 1, 'content-v1')",
                params![identity, relative_path],
            )
            .expect("seed current readiness target");
        if with_artifact {
            connection
                .execute(
                    "INSERT INTO analysis_jobs VALUES
                     (?1, 'source', 1, 'file', ?2, 'embedding_aspects', 'v1', 1, 'content-v1')",
                    params![sample_id, identity],
                )
                .expect("seed current readiness job");
            connection
                .execute(
                    "INSERT INTO source_readiness_artifacts VALUES
                     ('source', 'file', ?1, 'embedding_aspects', 'v1', 1, 'content-v1')",
                    [identity],
                )
                .expect("seed current readiness artifact");
        }
    }

    fn seed_obsolete_readiness(connection: &Connection) {
        connection
            .execute(
                "INSERT INTO analysis_jobs VALUES
                 ('source::deleted', 'source', 1, 'file', 'deleted-identity',
                  'embedding_aspects', 'v1', 1, 'deleted-content')",
                [],
            )
            .expect("seed obsolete readiness job");
        connection
            .execute(
                "INSERT INTO source_readiness_artifacts VALUES
                 ('source', 'file', 'deleted-identity', 'embedding_aspects',
                  'v1', 1, 'deleted-content')",
                [],
            )
            .expect("seed obsolete readiness artifact");
    }

    fn manifest_entry(sample_id: &str, embedding: Vec<f32>) -> ExactSimilarityManifestEntry {
        ExactSimilarityManifestEntry {
            sample_id: sample_id.to_string(),
            embedding,
        }
    }

    fn axis_embedding(axis: usize, value: f32) -> Vec<f32> {
        let mut embedding = vec![0.0; similarity::SIMILARITY_DIM];
        embedding[axis] = value;
        embedding
    }

    fn cluster_config() -> HdbscanConfig {
        HdbscanConfig {
            min_cluster_size: 10,
            min_samples: None,
            allow_single_cluster: false,
        }
    }

    fn request(artifact_generation: &str) -> ExactSimilarityArtifactRequest<'_> {
        ExactSimilarityArtifactRequest {
            source_id: "source",
            source_sample_id_prefix: "source::",
            model_id: similarity::SIMILARITY_MODEL_ID,
            layout_version: "layout-v1",
            artifact_contract_version: "bundle-v1",
            artifact_generation,
            cluster_config: cluster_config(),
        }
    }

    fn ann_meta_count(connection: &Connection) -> i64 {
        connection
            .query_row(
                "SELECT COALESCE(MAX(count), 0) FROM ann_index_meta",
                [],
                |row| row.get(0),
            )
            .expect("read ANN metadata count")
    }

    fn table_count(connection: &Connection, table: &str) -> i64 {
        connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count exact publication table")
    }

    fn assert_exact_membership(connection: &Connection, table: &str, expected: i64) {
        let actual = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("count exact artifact rows");
        assert_eq!(actual, expected);
        let deleted = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE sample_id = 'source::deleted'"),
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("count stale artifact rows");
        assert_eq!(deleted, 0);
    }

    fn nearest_id(connection: &Connection, embedding: &[f32]) -> String {
        ann_index::find_similar_for_embedding(connection, embedding, 1)
            .expect("query exact ANN")
            .into_iter()
            .next()
            .expect("nearest ANN result")
            .sample_id
    }
}
