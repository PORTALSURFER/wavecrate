use super::*;

/// Execute a background browser auto-rename batch, collecting renamed, skipped, and failed items.
pub(crate) fn run_sample_auto_rename_job(
    source: SampleSource,
    requests: Vec<SampleAutoRenameRequest>,
    cancel: Arc<AtomicBool>,
    progress: Option<FileOpProgressSender>,
) -> SampleAutoRenameResult {
    let started_at = std::time::Instant::now();
    let source_id = source.id.clone();
    let requested_paths = requests
        .iter()
        .map(|request| request.old_relative.clone())
        .collect::<Vec<_>>();
    let db = match crate::sample_sources::SourceDatabase::open_with_role(
        &source.root,
        crate::sample_sources::SourceDatabaseConnectionRole::JobWorker,
    ) {
        Ok(db) => db,
        Err(err) => {
            return SampleAutoRenameResult {
                source_id,
                requested_paths: requested_paths.clone(),
                renamed: Vec::new(),
                skipped: Vec::new(),
                errors: requested_paths
                    .into_iter()
                    .map(|path| (path, format!("Database unavailable: {err}")))
                    .collect(),
            };
        }
    };
    let mut result = AutoRenameBatchResult::new(source_id, requested_paths);
    for (index, request) in requests.into_iter().enumerate() {
        if !rename_one_auto_rename_request(
            &source,
            &db,
            request,
            index,
            &cancel,
            progress.as_ref(),
            &mut result,
        ) {
            break;
        }
    }
    let result = result.into_job_result();
    record_auto_rename_worker_latency(&result, started_at);
    result
}

fn rename_one_auto_rename_request(
    source: &SampleSource,
    db: &crate::sample_sources::SourceDatabase,
    request: SampleAutoRenameRequest,
    completed: usize,
    cancel: &Arc<AtomicBool>,
    progress: Option<&FileOpProgressSender>,
    result: &mut AutoRenameBatchResult,
) -> bool {
    emit_auto_rename_item_progress(
        progress,
        completed,
        None,
        SampleAutoRenameProgress::Active {
            old_relative: request.old_relative.clone(),
        },
    );
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        record_auto_rename_cancel(progress, completed, request, result);
        return false;
    }
    if request.old_relative == request.new_relative {
        mark_unchanged_auto_rename_request(db, request, completed, progress, result);
        return true;
    }
    rename_changed_auto_rename_request(source, db, request, completed, progress, result);
    true
}

fn record_auto_rename_cancel(
    progress: Option<&FileOpProgressSender>,
    completed: usize,
    request: SampleAutoRenameRequest,
    result: &mut AutoRenameBatchResult,
) {
    emit_auto_rename_item_progress(
        progress,
        completed,
        Some(format!("Cancelled at {}", request.old_relative.display())),
        SampleAutoRenameProgress::Failed {
            old_relative: request.old_relative.clone(),
            error: String::from("Rename cancelled"),
        },
    );
    result
        .errors
        .push((request.old_relative, String::from("Rename cancelled")));
}

fn mark_unchanged_auto_rename_request(
    db: &crate::sample_sources::SourceDatabase,
    request: SampleAutoRenameRequest,
    completed: usize,
    progress: Option<&FileOpProgressSender>,
    result: &mut AutoRenameBatchResult,
) {
    match mark_sample_tag_named_with_db(db, &request.old_relative, request.tag_named).and_then(
        |entry| {
            entry.ok_or_else(|| format!("Sample not found: {}", request.old_relative.display()))
        },
    ) {
        Ok(entry) => record_auto_rename_success(request, entry, completed, progress, result),
        Err(err) => {
            record_auto_rename_error(request.old_relative, err, completed, progress, result)
        }
    }
}

fn rename_changed_auto_rename_request(
    source: &SampleSource,
    db: &crate::sample_sources::SourceDatabase,
    request: SampleAutoRenameRequest,
    completed: usize,
    progress: Option<&FileOpProgressSender>,
    result: &mut AutoRenameBatchResult,
) {
    let old_absolute = source.root.join(&request.old_relative);
    let rename_result = rename::perform_sample_rename_with_db(
        source,
        db,
        &old_absolute,
        &request.old_relative,
        &request.new_relative,
        request.tag,
        RenameLoopedMetadata::RequestSnapshot(request.looped),
        request.locked,
        request.last_played_at,
        request.sound_type,
        request.user_tag.clone(),
        Some(request.tag_named),
    );
    match rename_result {
        Ok(entry) => record_auto_rename_success(request, entry, completed, progress, result),
        Err(err) if err.contains("already exists") => {
            record_auto_rename_skip(request.old_relative, err, completed, progress, result);
        }
        Err(err) => {
            record_auto_rename_error(request.old_relative, err, completed, progress, result)
        }
    }
}

fn record_auto_rename_success(
    request: SampleAutoRenameRequest,
    entry: WavEntry,
    completed: usize,
    progress: Option<&FileOpProgressSender>,
    result: &mut AutoRenameBatchResult,
) {
    emit_auto_rename_item_progress(
        progress,
        completed + 1,
        Some(format!("Renamed {}", request.new_relative.display())),
        SampleAutoRenameProgress::Completed {
            old_relative: request.old_relative.clone(),
            new_relative: request.new_relative.clone(),
        },
    );
    result.renamed.push(SampleAutoRenameSuccess {
        old_relative: request.old_relative,
        new_relative: request.new_relative,
        entry,
        resume_playback: request.resume_playback,
        resume_looped: request.resume_looped,
        resume_start_override: request.resume_start_override,
    });
}

fn record_auto_rename_skip(
    old_relative: PathBuf,
    err: String,
    completed: usize,
    progress: Option<&FileOpProgressSender>,
    result: &mut AutoRenameBatchResult,
) {
    emit_auto_rename_item_progress(
        progress,
        completed + 1,
        Some(format!("Skipped {}", old_relative.display())),
        SampleAutoRenameProgress::Skipped {
            old_relative: old_relative.clone(),
            reason: err.clone(),
        },
    );
    result.skipped.push((old_relative, err));
}

fn record_auto_rename_error(
    old_relative: PathBuf,
    err: String,
    completed: usize,
    progress: Option<&FileOpProgressSender>,
    result: &mut AutoRenameBatchResult,
) {
    emit_auto_rename_item_progress(
        progress,
        completed + 1,
        Some(format!("Failed {}", old_relative.display())),
        SampleAutoRenameProgress::Failed {
            old_relative: old_relative.clone(),
            error: err.clone(),
        },
    );
    result.errors.push((old_relative, err));
}

fn emit_auto_rename_item_progress(
    progress: Option<&FileOpProgressSender>,
    completed: usize,
    detail: Option<String>,
    item: SampleAutoRenameProgress,
) {
    if let Some(progress) = progress {
        progress.auto_rename_progress(completed, detail, item);
    }
}

struct AutoRenameBatchResult {
    source_id: SourceId,
    requested_paths: Vec<PathBuf>,
    renamed: Vec<SampleAutoRenameSuccess>,
    skipped: Vec<(PathBuf, String)>,
    errors: Vec<(PathBuf, String)>,
}

impl AutoRenameBatchResult {
    fn new(source_id: SourceId, requested_paths: Vec<PathBuf>) -> Self {
        Self {
            source_id,
            requested_paths,
            renamed: Vec::new(),
            skipped: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn into_job_result(self) -> SampleAutoRenameResult {
        SampleAutoRenameResult {
            source_id: self.source_id,
            requested_paths: self.requested_paths,
            renamed: self.renamed,
            skipped: self.skipped,
            errors: self.errors,
        }
    }
}

fn mark_sample_tag_named_with_db(
    db: &crate::sample_sources::SourceDatabase,
    relative_path: &Path,
    tag_named: bool,
) -> Result<Option<WavEntry>, String> {
    db.set_tag_named(relative_path, tag_named)
        .map_err(|err| format!("Failed to mark tag-name status: {err}"))?;
    db.entry_for_path(relative_path)
        .map_err(|err| format!("Failed to reload tag-name marker: {err}"))
}

fn record_auto_rename_worker_latency(
    #[cfg_attr(not(test), allow(unused_variables))] result: &SampleAutoRenameResult,
    #[cfg_attr(not(test), allow(unused_variables))] started_at: std::time::Instant,
) {
    #[cfg(test)]
    crate::app::controller::batch_latency::record(
        crate::app::controller::batch_latency::BatchLatencySample::new(
            crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenameWorker,
            result.requested_paths.len(),
            started_at.elapsed(),
        )
        .with_detail_count(result.renamed.len() + result.skipped.len() + result.errors.len()),
    );
}
