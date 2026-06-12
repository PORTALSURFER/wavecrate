use crate::app::controller::library::analysis_jobs::types::AnalysisProgress;

pub(super) fn decode_progress_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AnalysisProgress> {
    let pending = row.get::<_, i64>(0)?.max(0) as usize;
    let running = row.get::<_, i64>(1)?.max(0) as usize;
    let done = row.get::<_, i64>(2)?.max(0) as usize;
    let failed = row.get::<_, i64>(3)?.max(0) as usize;
    Ok(AnalysisProgress {
        pending,
        running,
        done,
        failed,
        samples_total: pending + running + done + failed,
        samples_pending_or_running: pending + running,
    })
}
