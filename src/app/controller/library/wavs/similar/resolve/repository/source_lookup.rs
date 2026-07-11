//! Controller/source lookup helpers for similarity resolution.

use crate::app::controller::AppController;
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::SourceId;

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
) -> Result<analysis_jobs::AnalysisReadSession, String> {
    let source = controller
        .library
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found".to_string())?;
    analysis_jobs::open_source_db_ui_read(&source.root)
}
