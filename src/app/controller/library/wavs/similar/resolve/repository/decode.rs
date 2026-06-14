//! Feature/blob decoding helpers reused by batched similarity loaders.

use super::super::FEATURE_RMS_INDEX;

pub(super) fn decode_similarity_feature_metrics(
    blob: &[u8],
    feat_version: i64,
) -> Result<super::loaders::SimilarityFeatureMetrics, String> {
    if feat_version == wavecrate_analysis::FEATURE_VERSION_V1 {
        let light_dsp = decode_f32_prefix(blob, wavecrate_analysis::LIGHT_DSP_VECTOR_LEN)?;
        let rms = decode_feature_rms(blob, feat_version)?;
        return Ok(super::loaders::SimilarityFeatureMetrics {
            light_dsp: Some(super::super::normalize_l2(light_dsp)),
            rms,
        });
    }

    let features = wavecrate_analysis::decode_f32_le_blob(blob)?;
    let rms = features.get(FEATURE_RMS_INDEX).copied();
    let light_dsp =
        wavecrate_analysis::light_dsp_from_features_v1(&features).map(super::super::normalize_l2);
    Ok(super::loaders::SimilarityFeatureMetrics { light_dsp, rms })
}

pub(super) fn decode_feature_rms(blob: &[u8], feat_version: i64) -> Result<Option<f32>, String> {
    if feat_version == wavecrate_analysis::FEATURE_VERSION_V1 {
        return decode_f32_at(blob, FEATURE_RMS_INDEX).map(Some);
    }
    let features = wavecrate_analysis::decode_f32_le_blob(blob)?;
    Ok(features.get(FEATURE_RMS_INDEX).copied())
}

pub(super) fn decode_light_dsp_blob(blob: &[u8]) -> Result<Vec<f32>, String> {
    let decoded = wavecrate_analysis::decode_f32_le_blob(blob)?;
    Ok(super::super::normalize_l2(decoded))
}

pub(super) fn feature_metric_column_missing(err: &rusqlite::Error) -> bool {
    err.to_string().contains("no such column")
}

pub(super) fn placeholder_list(start_index: usize, count: usize) -> String {
    (0..count)
        .map(|offset| format!("?{}", start_index + offset))
        .collect::<Vec<_>>()
        .join(", ")
}

fn decode_f32_prefix(blob: &[u8], count: usize) -> Result<Vec<f32>, String> {
    (0..count).map(|index| decode_f32_at(blob, index)).collect()
}

fn decode_f32_at(blob: &[u8], index: usize) -> Result<f32, String> {
    if !blob.len().is_multiple_of(4) {
        return Err("Feature blob length is not a multiple of 4 bytes".to_string());
    }
    let start = index.saturating_mul(4);
    let end = start.saturating_add(4);
    let Some(bytes) = blob.get(start..end) else {
        return Err(format!("Feature blob missing value at index {index}"));
    };
    Ok(f32::from_le_bytes(bytes.try_into().map_err(|_| {
        format!("Feature blob missing value at index {index}")
    })?))
}
