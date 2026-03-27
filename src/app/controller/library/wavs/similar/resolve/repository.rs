//! Source/DB lookup helpers for similarity resolution.

use crate::app::controller::AppController;
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::SourceId;
use rusqlite::{OptionalExtension, params};

use super::super::FEATURE_RMS_INDEX;

/// Resolve the sample identifier for one visible browser row.
pub(crate) fn resolve_sample_id_for_visible_row(
    controller: &mut AppController,
    visible_row: usize,
) -> Result<(String, usize), String> {
    let source_id = resolve_selected_source(controller)?;
    let entry_index = resolve_visible_row_index(controller, visible_row)?;
    let sample_id = resolve_sample_id_for_entry(controller, &source_id, entry_index)?;
    Ok((sample_id, entry_index))
}

fn resolve_selected_source(controller: &AppController) -> Result<SourceId, String> {
    controller
        .selection_state
        .ctx
        .selected_source
        .clone()
        .ok_or_else(|| "No active source selected".to_string())
}

fn resolve_visible_row_index(
    controller: &AppController,
    visible_row: usize,
) -> Result<usize, String> {
    controller
        .ui
        .browser
        .viewport
        .visible
        .get(visible_row)
        .ok_or_else(|| "Selected row is out of range".to_string())
}

fn resolve_sample_id_for_entry(
    controller: &mut AppController,
    source_id: &SourceId,
    entry_index: usize,
) -> Result<String, String> {
    let entry = controller
        .wav_entry(entry_index)
        .ok_or_else(|| "Sample entry missing".to_string())?;
    Ok(analysis_jobs::build_sample_id(
        source_id.as_str(),
        &entry.relative_path,
    ))
}

/// Open the selected source DB for similarity lookup.
pub(crate) fn open_source_db_for_id(
    controller: &AppController,
    source_id: &SourceId,
) -> Result<rusqlite::Connection, String> {
    let source = controller
        .library
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found".to_string())?;
    analysis_jobs::open_source_db(&source.root)
}

/// Load the lightweight DSP vector used to refine ANN similarity results.
pub(crate) fn load_light_dsp_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<Vec<f32>>, String> {
    let blob: Option<Vec<u8>> = conn
        .query_row(
            "SELECT vec_blob FROM features WHERE sample_id = ?1",
            [sample_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Load features failed: {err}"))?;
    let Some(blob) = blob else {
        return Ok(None);
    };
    let features = crate::analysis::decode_f32_le_blob(&blob)?;
    let light = crate::analysis::light_dsp_from_features_v1(&features);
    Ok(light.map(super::normalize_l2))
}

/// Load the RMS feature value used for duplicate/silence filtering.
pub(crate) fn load_rms_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<f32>, String> {
    let blob: Option<Vec<u8>> = conn
        .query_row(
            "SELECT vec_blob FROM features WHERE sample_id = ?1",
            [sample_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Load features failed: {err}"))?;
    let Some(blob) = blob else {
        return Ok(None);
    };
    let features = crate::analysis::decode_f32_le_blob(&blob)?;
    if features.len() <= FEATURE_RMS_INDEX {
        return Ok(None);
    }
    Ok(Some(features[FEATURE_RMS_INDEX]))
}

/// Load the persisted similarity embedding for one sample.
pub(crate) fn load_embedding_for_sample(
    conn: &rusqlite::Connection,
    sample_id: &str,
) -> Result<Option<Vec<f32>>, String> {
    let blob: Option<Vec<u8>> = conn
        .query_row(
            "SELECT vec FROM embeddings WHERE sample_id = ?1 AND model_id = ?2",
            params![sample_id, crate::analysis::similarity::SIMILARITY_MODEL_ID],
            |row| row.get(0),
        )
        .optional()
        .map_err(|err| format!("Load embedding failed: {err}"))?;
    let Some(blob) = blob else {
        return Ok(None);
    };
    crate::analysis::decode_f32_le_blob(&blob).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::dummy_controller;
    use crate::app::state::VisibleRows;

    #[test]
    fn resolve_sample_id_for_visible_row_errors_on_empty_visible_rows() {
        let (mut controller, _source) = dummy_controller();
        controller.ui.browser.viewport.visible = VisibleRows::List(Vec::new().into());
        let err = resolve_sample_id_for_visible_row(&mut controller, 0).unwrap_err();
        assert_eq!(err, "Selected row is out of range");
    }

    #[test]
    fn resolve_sample_id_for_visible_row_errors_on_missing_entry() {
        let (mut controller, _source) = dummy_controller();
        controller.ui.browser.viewport.visible = VisibleRows::List(vec![0].into());
        let err = resolve_sample_id_for_visible_row(&mut controller, 0).unwrap_err();
        assert_eq!(err, "Sample entry missing");
    }
}
