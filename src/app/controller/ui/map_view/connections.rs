//! Connection helpers for map-view queries and background layout jobs.

use crate::app::controller::library::analysis_jobs;
use rusqlite::Connection;

use super::*;

/// Return a cached per-source map-query connection, opening it on first use.
pub(super) fn open_cached_source_db<'a>(
    controller: &'a mut AppController,
    source_id: Option<&SourceId>,
) -> Result<&'a mut Connection, String> {
    let source_id = source_id
        .ok_or_else(|| "No source selected".to_string())?
        .clone();
    let source_root = controller
        .library
        .sources
        .iter()
        .find(|source| source.id == source_id)
        .map(|source| source.root.clone())
        .ok_or_else(|| "Source not found".to_string())?;
    if !controller
        .runtime
        .map_query_connections
        .contains_key(&source_id)
    {
        let conn = analysis_jobs::open_source_db(&source_root)?;
        controller
            .runtime
            .map_query_connections
            .insert(source_id.clone(), conn);
    }
    controller
        .runtime
        .map_query_connections
        .get_mut(&source_id)
        .ok_or_else(|| "Map query connection missing after open".to_string())
}

pub(super) fn open_source_db_for_id(source_id: &SourceId) -> Result<Connection, String> {
    let state = crate::sample_sources::library::load().map_err(|err| err.to_string())?;
    let source = state
        .sources
        .iter()
        .find(|source| &source.id == source_id)
        .ok_or_else(|| "Source not found".to_string())?;
    analysis_jobs::open_source_db(&source.root)
}
