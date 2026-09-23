//! Database work for the unsupported-files dialog, dispatched through BusinessRuntime.

use std::{
    collections::BTreeSet,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use wavecrate::sample_sources::{
    SourceDatabase, SourceDatabaseConnectionRole,
    readiness::{ReadinessClassification, ReadinessScopeKind, ReadinessView},
};

pub(super) fn load_unsupported_files(
    source_root: PathBuf,
    database_root: PathBuf,
    source_id: String,
) -> Result<Vec<PathBuf>, String> {
    let connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source_root,
        &database_root,
        SourceDatabaseConnectionRole::BackgroundRead,
    )
    .map_err(|error| format!("open source readiness database: {error}"))?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("read current time: {error}"))?
        .as_secs() as i64;
    let snapshot = ReadinessView::new(&connection)
        .reconcile(&source_id, now)
        .map_err(|error| format!("read source readiness: {error}"))?;
    let mut paths = BTreeSet::new();
    for entry in snapshot.entries {
        if entry.target.scope_kind != ReadinessScopeKind::File
            || !matches!(
                entry.classification,
                ReadinessClassification::Unsupported { .. }
            )
        {
            continue;
        }
        if let Some(relative_path) = entry.target.relative_path {
            paths.insert(source_root.join(relative_path));
        }
    }
    Ok(paths.into_iter().collect())
}
