//! Runtime state for retained map-query database connections.

use crate::app::controller::library::analysis_jobs::AnalysisReadSession;
use crate::sample_sources::SourceId;
use std::collections::HashMap;

/// Reused map-query SQLite connections keyed by source id.
#[derive(Default)]
pub(crate) struct MapRuntimeState {
    pub(crate) query_connections: HashMap<SourceId, AnalysisReadSession>,
}
