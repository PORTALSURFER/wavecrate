use rusqlite::params;

use super::{ReadinessError, ReadinessStore, ReadinessView};

/// One current embedding artifact selected for similarity publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessEmbeddingArtifactTarget {
    /// Stable file identity.
    pub scope_id: String,
    /// Current source-relative path.
    pub relative_path: String,
    /// Required embedding contract version.
    pub required_version: String,
    /// Source generation.
    pub source_generation: i64,
    /// Exact content generation.
    pub content_generation: String,
}

/// Analysis payload contract required for an exact similarity publication.
#[derive(Clone, Copy, Debug)]
pub struct ReadinessSimilarityPayloadContract<'a> {
    analysis_version: &'a str,
    embedding_model_id: &'a str,
    feature_version: i64,
    embedding_dim: i64,
    embedding_dtype: &'a str,
    aspect_model_id: &'a str,
    aspect_dim: i64,
    aspect_dtype: &'a str,
}

impl<'a> ReadinessSimilarityPayloadContract<'a> {
    /// Build the exact analysis contract required by a similarity publisher.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        analysis_version: &'a str,
        embedding_model_id: &'a str,
        feature_version: i64,
        embedding_dim: i64,
        embedding_dtype: &'a str,
        aspect_model_id: &'a str,
        aspect_dim: i64,
        aspect_dtype: &'a str,
    ) -> Self {
        Self {
            analysis_version,
            embedding_model_id,
            feature_version,
            embedding_dim,
            embedding_dtype,
            aspect_model_id,
            aspect_dim,
            aspect_dtype,
        }
    }
}

/// Typed request for the exact embedding payloads owned by one readiness generation.
#[derive(Clone, Copy, Debug)]
pub struct ReadinessSimilarityManifestRequest<'a> {
    source_id: &'a str,
    source_generation: i64,
    contract: ReadinessSimilarityPayloadContract<'a>,
}

impl<'a> ReadinessSimilarityManifestRequest<'a> {
    /// Bind an analysis payload contract to one source generation.
    pub fn new(
        source_id: &'a str,
        source_generation: i64,
        contract: ReadinessSimilarityPayloadContract<'a>,
    ) -> Self {
        Self {
            source_id,
            source_generation,
            contract,
        }
    }
}

/// One exact embedding payload selected through the readiness publication fence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessSimilarityManifestRow {
    /// Stable file identity used in the membership generation.
    pub scope_id: String,
    /// Current source-relative path.
    pub relative_path: String,
    /// Exact content generation.
    pub content_generation: String,
    /// Stored embedding dimension.
    pub embedding_dim: i64,
    /// Stored little-endian embedding payload.
    pub embedding: Vec<u8>,
}

/// Complete readiness-owned selection for exact similarity publication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessSimilarityManifest {
    /// Number of eligible embedding targets in the desired readiness set.
    pub target_count: usize,
    /// Targets whose readiness artifact and analysis payload are both exact.
    pub rows: Vec<ReadinessSimilarityManifestRow>,
}

impl ReadinessStore<'_> {
    /// Check the exact source-level similarity publication fence against durable readiness state.
    pub fn similarity_publication_is_current(
        &mut self,
        source_id: &str,
        source_generation: i64,
        artifact_version: &str,
        membership_generation: &str,
        manifest_generation: i64,
    ) -> Result<bool, ReadinessError> {
        ReadinessView::new(self.connection).similarity_publication_is_current(
            source_id,
            source_generation,
            artifact_version,
            membership_generation,
            manifest_generation,
        )
    }

    /// Whether indexed file targets still await a committed content generation.
    pub fn has_pending_file_content(&mut self, source_id: &str) -> Result<bool, ReadinessError> {
        self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM source_readiness_targets WHERE source_id = ?1 AND scope_kind = 'file' AND stage = 'indexed_identity' AND content_generation LIKE 'pending-%')",
            [source_id],
            |row| row.get(0),
        ).map_err(Into::into)
    }

    /// Load current embedding artifact targets for a similarity generation.
    pub fn embedding_artifact_targets(
        &mut self,
        source_id: &str,
        source_generation: i64,
    ) -> Result<Vec<ReadinessEmbeddingArtifactTarget>, ReadinessError> {
        let mut statement = self.connection.prepare(
            "SELECT target.scope_id, target.relative_path, target.required_version, target.source_generation, target.content_generation FROM source_readiness_targets AS target JOIN source_readiness_sources AS source ON source.source_id = target.source_id AND source.source_generation = ?2 AND source.availability = 'active' JOIN source_readiness_artifacts AS artifact ON artifact.source_id = target.source_id AND artifact.scope_kind = target.scope_kind AND artifact.scope_id = target.scope_id AND artifact.stage = target.stage AND artifact.artifact_version = target.required_version AND artifact.content_generation = target.content_generation WHERE target.source_id = ?1 AND target.scope_kind = 'file' AND target.stage = 'embedding_aspects' AND target.eligibility = 'eligible'",
        )?;
        statement
            .query_map(params![source_id, source_generation], |row| {
                Ok(ReadinessEmbeddingArtifactTarget {
                    scope_id: row.get(0)?,
                    relative_path: row.get(1)?,
                    required_version: row.get(2)?,
                    source_generation: row.get(3)?,
                    content_generation: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    /// Select exact similarity payloads through current targets and artifacts.
    pub fn similarity_manifest(
        &mut self,
        request: ReadinessSimilarityManifestRequest<'_>,
    ) -> Result<ReadinessSimilarityManifest, ReadinessError> {
        let target_count = self.connection.query_row(
            "SELECT COUNT(*)
             FROM source_readiness_targets
             WHERE source_id = ?1
               AND scope_kind = 'file'
               AND stage = 'embedding_aspects'
               AND eligibility = 'eligible'",
            [request.source_id],
            |row| row.get::<_, i64>(0),
        )?;
        let mut statement = self.connection.prepare(
            "SELECT target.scope_id, target.relative_path, target.content_generation,
                    embedding.dim, embedding.vec
             FROM source_readiness_targets AS target
             JOIN source_readiness_sources AS source
               ON source.source_id = target.source_id
              AND source.source_generation = ?2
              AND source.availability = 'active'
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
               AND target.eligibility = 'eligible'
               AND feature.feat_version = ?6
               AND embedding.dim = ?7
               AND embedding.dtype = ?8
               AND embedding.l2_normed = 1
               AND aspects.dim = ?9
               AND aspects.dtype = ?10
               AND aspects.l2_normed = 1
             ORDER BY target.relative_path",
        )?;
        let contract = request.contract;
        let rows = statement
            .query_map(
                params![
                    request.source_id,
                    request.source_generation,
                    contract.embedding_model_id,
                    contract.analysis_version,
                    contract.aspect_model_id,
                    contract.feature_version,
                    contract.embedding_dim,
                    contract.embedding_dtype,
                    contract.aspect_dim,
                    contract.aspect_dtype,
                ],
                |row| {
                    Ok(ReadinessSimilarityManifestRow {
                        scope_id: row.get(0)?,
                        relative_path: row.get(1)?,
                        content_generation: row.get(2)?,
                        embedding_dim: row.get(3)?,
                        embedding: row.get(4)?,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ReadinessSimilarityManifest {
            target_count: usize::try_from(target_count).unwrap_or(usize::MAX),
            rows,
        })
    }
}
