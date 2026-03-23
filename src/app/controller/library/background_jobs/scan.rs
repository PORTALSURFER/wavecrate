use super::progress;
use super::*;
use crate::app::state::ProgressTaskKind;
use crate::sample_sources::scanner::ChangedSample;

/// Apply incremental scan progress to the shared progress UI.
pub(crate) fn handle_scan_progress(
    controller: &mut AppController,
    completed: usize,
    detail: Option<String>,
) {
    let detail = match detail {
        Some(detail) if !detail.is_empty() => {
            format!("Scanned {completed} file(s)\n{detail}")
        }
        _ => format!("Scanned {completed} file(s)"),
    };
    progress::update_progress_detail(controller, ProgressTaskKind::Scan, completed, Some(detail));
}

/// Finalize scan state, refresh caches, and queue analysis follow-up work.
pub(crate) fn handle_scan_finished(controller: &mut AppController, result: ScanResult) {
    controller.runtime.jobs.clear_scan();
    if controller.ui.progress.task == Some(ProgressTaskKind::Scan) {
        controller.clear_progress();
    }
    let is_selected_source =
        Some(&result.source_id) == controller.selection_state.ctx.selected_source.as_ref();
    let is_auto = matches!(result.kind, ScanKind::Auto);
    let label = match result.mode {
        ScanMode::Quick => "Quick sync",
        ScanMode::Hard => "Hard sync",
    };
    match result.result {
        Ok(stats) => handle_successful_scan(
            controller,
            &result.source_id,
            label,
            is_selected_source,
            is_auto,
            stats,
        ),
        Err(crate::sample_sources::scanner::ScanError::Canceled) => {
            handle_scan_failure(
                controller,
                &result.source_id,
                label,
                is_selected_source,
                None,
            );
        }
        Err(err) => handle_scan_failure(
            controller,
            &result.source_id,
            label,
            is_selected_source,
            Some(err),
        ),
    }
}

fn handle_successful_scan(
    controller: &mut AppController,
    source_id: &SourceId,
    label: &str,
    is_selected_source: bool,
    is_auto: bool,
    stats: ScanStats,
) {
    let changed_samples = stats.changed_samples.clone();
    let scan_changed = !changed_samples.is_empty();
    let similarity_prep_active = controller
        .runtime
        .similarity_prep
        .as_ref()
        .is_some_and(|state| state.source_id == *source_id);

    report_successful_scan_status(
        controller,
        label,
        is_selected_source,
        is_auto,
        scan_changed,
        &stats,
    );
    invalidate_scan_caches(controller, source_id, is_selected_source);

    let source = controller
        .library
        .sources
        .iter()
        .find(|source| source.id == *source_id)
        .cloned();

    if scan_changed {
        if let Some(source) = source.clone() {
            spawn_changed_scan_enqueue(controller, source, changed_samples);
        }
    } else if let Some(source) = source.clone() {
        if similarity_prep_active {
            controller.handle_similarity_scan_finished(source_id, false);
            return;
        }
        spawn_unchanged_scan_backfill(controller, source);
    }

    if let Some(source) = source {
        spawn_duration_refresh(controller, source);
    }
    controller.handle_similarity_scan_finished(source_id, scan_changed);
}

fn report_successful_scan_status(
    controller: &mut AppController,
    label: &str,
    is_selected_source: bool,
    is_auto: bool,
    scan_changed: bool,
    stats: &ScanStats,
) {
    if is_selected_source && (!is_auto || scan_changed) {
        controller.set_status(
            format!(
                "{label} complete: {} added, {} updated, {} missing",
                stats.added, stats.updated, stats.missing
            ),
            StatusTone::Info,
        );
    }
}

fn invalidate_scan_caches(
    controller: &mut AppController,
    source_id: &SourceId,
    is_selected_source: bool,
) {
    let mut invalidator = source_cache_invalidator::SourceCacheInvalidator::new_from_state(
        &mut controller.cache,
        &mut controller.ui_cache,
        &mut controller.library.missing,
    );
    invalidator.invalidate_wav_related(source_id);
    if is_selected_source {
        controller.queue_wav_load();
    }
}

fn spawn_changed_scan_enqueue(
    controller: &mut AppController,
    source: SampleSource,
    changed_samples: Vec<ChangedSample>,
) {
    let tx = controller.runtime.jobs.message_sender();
    std::thread::spawn(move || {
        match analysis_jobs::enqueue_jobs_for_source(&source, &changed_samples) {
            Ok((inserted, progress)) => {
                let _ = tx.send(JobMessage::Analysis(
                    super::AnalysisJobMessage::EnqueueFinished { inserted, progress },
                ));
            }
            Err(err) => {
                let _ = tx.send(JobMessage::Analysis(
                    super::AnalysisJobMessage::EnqueueFailed(err),
                ));
            }
        }
    });
}

fn spawn_unchanged_scan_backfill(controller: &mut AppController, source: SampleSource) {
    let tx = controller.runtime.jobs.message_sender();
    std::thread::spawn(move || {
        match analysis_jobs::enqueue_jobs_for_source_backfill(&source) {
            Ok((inserted, progress)) => {
                let _ = tx.send(JobMessage::Analysis(
                    super::AnalysisJobMessage::EnqueueFinished { inserted, progress },
                ));
            }
            Err(err) => {
                let _ = tx.send(JobMessage::Analysis(
                    super::AnalysisJobMessage::EnqueueFailed(err),
                ));
            }
        }
        match analysis_jobs::enqueue_jobs_for_embedding_backfill(&source) {
            Ok((inserted, progress)) => {
                if inserted > 0 {
                    let _ = tx.send(JobMessage::Analysis(
                        super::AnalysisJobMessage::EmbeddingBackfillEnqueueFinished {
                            inserted,
                            progress,
                        },
                    ));
                }
            }
            Err(err) => {
                let _ = tx.send(JobMessage::Analysis(
                    super::AnalysisJobMessage::EmbeddingBackfillEnqueueFailed(err),
                ));
            }
        }
    });
}

fn spawn_duration_refresh(controller: &mut AppController, source: SampleSource) {
    let tx = controller.runtime.jobs.message_sender();
    std::thread::spawn(
        move || match analysis_jobs::update_missing_durations_for_source(&source) {
            Ok(updated) => {
                if updated > 0 {
                    let _ = tx.send(JobMessage::Analysis(
                        super::AnalysisJobMessage::DurationsUpdated {
                            source_id: source.id.clone(),
                            updated,
                        },
                    ));
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Duration probe after scan failed for {}: {err}",
                    source.id.as_str()
                );
            }
        },
    );
}

fn handle_scan_failure(
    controller: &mut AppController,
    source_id: &SourceId,
    label: &str,
    is_selected_source: bool,
    err: Option<crate::sample_sources::scanner::ScanError>,
) {
    if is_selected_source {
        let message = match err {
            Some(err) => format!("{label} failed: {err}"),
            None => format!("{label} canceled"),
        };
        let tone = if message.ends_with("canceled") {
            StatusTone::Warning
        } else {
            StatusTone::Error
        };
        controller.set_status(message, tone);
    }
    controller.cancel_similarity_prep(source_id);
}

#[cfg(test)]
mod tests;
