use crate::app::controller::library::analysis_jobs::db;

use super::super::support::now_epoch_seconds;

const FEATURE_RMS_INDEX: usize = 2;

/// Owned persistence payload for one decoded analysis result.
pub(crate) struct DecodedAnalysisWrite {
    pub(super) sample_id: String,
    pub(super) content_hash: String,
    pub(super) analysis_version: String,
    pub(super) duration_seconds: f32,
    pub(super) sample_rate: u32,
    pub(super) feature_blob: Vec<u8>,
    pub(super) light_dsp_blob: Option<Vec<u8>>,
    pub(super) rms: Option<f32>,
    pub(super) computed_at: i64,
    pub(super) embedding_blob: Vec<u8>,
    pub(super) embedding_created_at: i64,
    pub(super) needs_embedding_upsert: bool,
    pub(super) ann_embedding: Vec<f32>,
}

/// Precompute all SQL and ANN payloads for one decoded analysis result.
pub(crate) fn build_decoded_analysis_write(
    job: &db::ClaimedJob,
    decoded: wavecrate_analysis::AnalysisAudio,
    analysis_version: &str,
    needs_embedding_upsert: bool,
) -> Result<DecodedAnalysisWrite, String> {
    let content_hash = job
        .content_hash
        .clone()
        .ok_or_else(|| format!("Missing content_hash for analysis job {}", job.sample_id))?;
    let vector = wavecrate_analysis::compute_feature_vector_v1_for_decoded_audio(&decoded)?;
    let embedding = wavecrate_analysis::similarity::embedding_from_features(&vector)?;
    let feature_blob = wavecrate_analysis::vector::encode_f32_le_blob(&vector);
    let (light_dsp_blob, rms) = derive_similarity_metric_payloads(&vector);
    let computed_at = now_epoch_seconds();
    Ok(DecodedAnalysisWrite {
        sample_id: job.sample_id.clone(),
        content_hash,
        analysis_version: analysis_version.to_string(),
        duration_seconds: decoded.duration_seconds,
        sample_rate: decoded.sample_rate_used,
        feature_blob,
        light_dsp_blob,
        rms,
        computed_at,
        embedding_blob: wavecrate_analysis::vector::encode_f32_le_blob(&embedding),
        embedding_created_at: now_epoch_seconds(),
        needs_embedding_upsert,
        ann_embedding: embedding,
    })
}

fn derive_similarity_metric_payloads(features: &[f32]) -> (Option<Vec<u8>>, Option<f32>) {
    let light_dsp_blob = wavecrate_analysis::light_dsp_from_features_v1(features)
        .map(|light_dsp| wavecrate_analysis::vector::encode_f32_le_blob(&light_dsp));
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    (light_dsp_blob, rms)
}

impl DecodedAnalysisWrite {
    pub(super) fn metadata_update(&self) -> db::AnalysisMetadataUpdate<'_> {
        db::AnalysisMetadataUpdate {
            sample_id: &self.sample_id,
            content_hash: Some(&self.content_hash),
            duration_seconds: self.duration_seconds,
            sr_used: self.sample_rate,
            analysis_version: &self.analysis_version,
        }
    }

    pub(super) fn embedding_upsert(&self) -> db::EmbeddingUpsert<'_> {
        db::EmbeddingUpsert {
            sample_id: &self.sample_id,
            model_id: wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            dim: wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            dtype: wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &self.embedding_blob,
            created_at: self.embedding_created_at,
        }
    }

    pub(super) fn cached_features_upsert(&self) -> db::CachedFeaturesUpsert<'_> {
        db::CachedFeaturesUpsert {
            content_hash: &self.content_hash,
            analysis_version: &self.analysis_version,
            feat_version: wavecrate_analysis::vector::FEATURE_VERSION_V1,
            vec_blob: &self.feature_blob,
            light_dsp_blob: self.light_dsp_blob.as_deref(),
            rms: self.rms,
            computed_at: self.computed_at,
            duration_seconds: self.duration_seconds,
            sr_used: self.sample_rate,
        }
    }

    pub(super) fn cached_embedding_upsert(&self) -> db::CachedEmbeddingUpsert<'_> {
        db::CachedEmbeddingUpsert {
            content_hash: &self.content_hash,
            analysis_version: &self.analysis_version,
            model_id: wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
            dim: wavecrate_analysis::similarity::SIMILARITY_DIM as i64,
            dtype: wavecrate_analysis::similarity::SIMILARITY_DTYPE_F32,
            l2_normed: true,
            vec_blob: &self.embedding_blob,
            created_at: self.embedding_created_at,
        }
    }
}
