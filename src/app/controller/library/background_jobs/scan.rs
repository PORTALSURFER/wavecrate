use super::progress;
use super::*;
use crate::app::state::ProgressTaskKind;

/// Apply incremental scan progress to the shared progress UI.
pub(crate) fn handle_scan_progress(
    controller: &mut AppController,
    completed: usize,
    detail: Option<String>,
) {
    let detail = detail.and_then(|detail| {
        let trimmed = detail.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    progress::update_progress_detail(controller, ProgressTaskKind::Scan, completed, detail);
}

/// Finalize scan state, refresh caches, and narrowly scoped metadata probes.
pub(crate) fn handle_scan_finished(controller: &mut AppController, result: ScanResult) {
    controller.runtime.jobs.clear_scan();
    let is_selected_source =
        Some(&result.source_id) == controller.selection_state.ctx.selected_source.as_ref();
    let is_auto = matches!(result.kind, ScanKind::Auto);
    let label = match result.mode {
        ScanMode::Targeted => "Targeted sync",
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
            clear_scan_progress_if_active(controller);
            handle_scan_failure(
                controller,
                &result.source_id,
                label,
                is_selected_source,
                None,
            );
        }
        Err(err) => {
            clear_scan_progress_if_active(controller);
            handle_scan_failure(
                controller,
                &result.source_id,
                label,
                is_selected_source,
                Some(err),
            )
        }
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
    if !apply_selected_source_scan_deltas(controller, source_id, is_selected_source, &stats) {
        invalidate_scan_caches(controller, source_id, is_selected_source);
    }
    if is_selected_source {
        controller.refresh_selected_source_similarity_prep_status();
    }

    let source = controller
        .library
        .sources
        .iter()
        .find(|source| source.id == *source_id)
        .cloned();

    if let Some(source) = source {
        spawn_duration_refresh(controller, source);
    }
    if similarity_prep_active {
        controller.handle_similarity_scan_finished(source_id);
    }
    clear_scan_progress_if_active(controller);
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
        controller.set_background_status(
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

fn apply_selected_source_scan_deltas(
    controller: &mut AppController,
    source_id: &SourceId,
    is_selected_source: bool,
    stats: &ScanStats,
) -> bool {
    if !is_selected_source || !scan_is_small_known_delta(stats) {
        return false;
    }
    let Some(source) = controller
        .library
        .sources
        .iter()
        .find(|source| source.id == *source_id)
        .cloned()
    else {
        return false;
    };
    let Ok(db) = controller.database_for(&source) else {
        return false;
    };
    let mut updates = Vec::new();
    for renamed in &stats.renamed_samples {
        if !source_has_cached_path(controller, source_id, &renamed.old_relative_path) {
            continue;
        }
        let Ok(Some(entry)) = db.entry_for_path(&renamed.new_relative_path) else {
            return false;
        };
        updates.push((renamed.old_relative_path.clone(), entry));
    }
    for updated in &stats.updated_samples {
        if !source_has_cached_path(controller, source_id, &updated.relative_path) {
            continue;
        }
        let Ok(Some(entry)) = db.entry_for_path(&updated.relative_path) else {
            return false;
        };
        updates.push((updated.relative_path.clone(), entry));
    }
    drop(db);
    for (old_path, entry) in updates {
        controller.update_cached_entry(&source, &old_path, entry);
    }
    if !stats.updated_samples.is_empty() || stats.content_changed > 0 {
        controller.ui_cache.browser.features.remove(source_id);
        controller.ui_cache.browser.durations.remove(source_id);
        controller
            .ui_cache
            .browser
            .analysis_failures
            .remove(source_id);
    }
    controller.rebuild_missing_lookup_for_source(source_id);
    true
}

fn scan_is_small_known_delta(stats: &ScanStats) -> bool {
    let known_changes = stats.updated_samples.len() + stats.renamed_samples.len();
    if known_changes == 0 {
        return false;
    }
    if stats.added > 0 || stats.missing > 0 {
        return false;
    }
    if stats.renames_reconciled != stats.renamed_samples.len() {
        return false;
    }
    stats.updated <= known_changes
}

fn source_has_cached_path(
    controller: &AppController,
    source_id: &SourceId,
    path: &std::path::Path,
) -> bool {
    controller.wav_entries.lookup.contains_key(path)
        || controller
            .cache
            .wav
            .entries
            .get(source_id)
            .is_some_and(|cache| cache.lookup.contains_key(path))
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

fn clear_scan_progress_if_active(controller: &mut AppController) {
    controller.clear_progress_task(ProgressTaskKind::Scan);
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
        controller.set_background_status(message, tone);
    }
    controller.cancel_similarity_prep(source_id);
}

#[cfg(test)]
mod tests;
