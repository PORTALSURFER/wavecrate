//! Runtime state for retained map-query database connections.

use crate::sample_sources::SourceId;
use rusqlite::Connection;
use std::collections::HashMap;

/// Reused map-query SQLite connections keyed by source id.
#[derive(Default)]
pub(crate) struct MapRuntimeState {
    pub(crate) query_connections: HashMap<SourceId, Connection>,
}
