use super::*;

pub(in crate::app::controller::library::browser_controller::actions::metadata) fn run_background_auto_rename_request(
    snapshot: AutoRenameBackgroundRequest,
    cancel: Arc<AtomicBool>,
    progress: FileOpProgressSender,
) -> SampleAutoRenameResult {
    let source_id = snapshot.source.id.clone();
    let requested_paths = snapshot.paths.clone();
    match prepare_auto_rename_requests_from_snapshot(&snapshot, cancel.clone(), &progress) {
        Ok(requests) => {
            run_sample_auto_rename_job(snapshot.source, requests, cancel, Some(progress))
        }
        Err(err) => SampleAutoRenameResult {
            source_id,
            requested_paths: requested_paths.clone(),
            renamed: Vec::new(),
            skipped: Vec::new(),
            errors: vec![(
                requested_paths
                    .first()
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from(".")),
                err,
            )],
        },
    }
}

fn prepare_auto_rename_requests_from_snapshot(
    snapshot: &AutoRenameBackgroundRequest,
    cancel: Arc<AtomicBool>,
    progress: &FileOpProgressSender,
) -> Result<Vec<SampleAutoRenameRequest>, String> {
    let started_at = Instant::now();
    let db = crate::sample_sources::SourceDatabase::open_read_only(&snapshot.source.root)
        .map_err(|err| format!("Database unavailable: {err}"))?;
    let bpm_sample_ids = snapshot
        .paths
        .iter()
        .map(|path| {
            crate::app::controller::library::analysis_jobs::build_sample_id(
                snapshot.source.id.as_str(),
                path,
            )
        })
        .collect::<Vec<_>>();
    let bpm_lookup = db
        .bpms_for_sample_ids(&bpm_sample_ids)
        .map_err(|err| format!("Failed to read BPM metadata: {err}"))?;
    let mut requests = Vec::with_capacity(snapshot.paths.len());
    let mut reserved_targets = HashSet::new();
    for (relative_path, sample_id) in snapshot.paths.iter().zip(bpm_sample_ids.iter()) {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            progress.progress(
                0,
                Some(format!(
                    "Cancelled while planning {}",
                    relative_path.display()
                )),
            );
            return Err(String::from("Rename cancelled"));
        }
        progress.progress(0, Some(format!("Planning {}", relative_path.display())));
        let db_entry = snapshot_entry(snapshot, &db, relative_path)?;
        let normal_tags = snapshot_normal_tags(snapshot, &db, relative_path)?;
        let sound_type = db_entry
            .sound_type
            .or_else(|| infer_sound_type_from_path(relative_path));
        let bpm = snapshot_bpm(snapshot, relative_path, sample_id.as_str(), &bpm_lookup);
        let stem = build_auto_rename_stem(&AutoRenameInput {
            identifier: snapshot.identifier.clone(),
            looped: db_entry.looped,
            sound_type,
            user_tag: db_entry.user_tag.clone(),
            normal_tags,
            bpm,
        });
        let new_relative = target::resolve_auto_rename_target_for_worker(
            &snapshot.source.root,
            relative_path,
            stem.tagged_basename.as_deref(),
            &stem.fallback_identifier,
            &mut reserved_targets,
        )?;
        requests.push(SampleAutoRenameRequest {
            old_relative: relative_path.clone(),
            new_relative,
            tag: db_entry.tag,
            looped: db_entry.looped,
            locked: db_entry.locked,
            sound_type,
            user_tag: db_entry.user_tag,
            tag_named: stem.tag_based,
            last_played_at: db_entry.last_played_at,
            resume_playback: snapshot.is_playing
                && snapshot
                    .loaded_relative
                    .as_ref()
                    .is_some_and(|loaded| loaded == relative_path),
            resume_looped: snapshot.resume_looped,
            resume_start_override: snapshot.resume_start_override,
        });
    }
    let elapsed = started_at.elapsed();
    logging::record_auto_rename_prepare_latency(requests.len(), elapsed);
    log_background_auto_rename_preparation(&snapshot.source, requests.len(), elapsed);
    logging::log_prepared_auto_rename_requests(&snapshot.source, &requests, elapsed, "background");
    Ok(requests)
}

fn snapshot_entry(
    snapshot: &AutoRenameBackgroundRequest,
    db: &crate::sample_sources::SourceDatabase,
    relative_path: &Path,
) -> Result<WavEntry, String> {
    match snapshot
        .metadata
        .get(relative_path)
        .and_then(|metadata| metadata.entry.clone())
    {
        Some(entry) => Ok(entry),
        None => db
            .entry_for_path(relative_path)
            .map_err(|err| format!("Failed to read sample metadata: {err}"))?
            .ok_or_else(|| format!("Sample not found: {}", relative_path.display())),
    }
}

fn snapshot_normal_tags(
    snapshot: &AutoRenameBackgroundRequest,
    db: &crate::sample_sources::SourceDatabase,
    relative_path: &Path,
) -> Result<Vec<String>, String> {
    match snapshot
        .metadata
        .get(relative_path)
        .and_then(|metadata| metadata.normal_tags.clone())
    {
        Some(tags) => Ok(tags),
        None => Ok(db
            .tags_for_path(relative_path)
            .map_err(|err| format!("Failed to read normal tags: {err}"))?
            .into_iter()
            .map(|tag| tag.display_label)
            .collect()),
    }
}

fn snapshot_bpm(
    snapshot: &AutoRenameBackgroundRequest,
    relative_path: &Path,
    sample_id: &str,
    bpm_lookup: &HashMap<String, Option<f32>>,
) -> Option<f32> {
    snapshot
        .metadata
        .get(relative_path)
        .and_then(|metadata| metadata.bpm)
        .or_else(|| bpm_lookup.get(sample_id).copied().flatten())
}

fn infer_sound_type_from_path(
    relative_path: &Path,
) -> Option<crate::sample_sources::SampleSoundType> {
    relative_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(crate::sample_sources::SampleSoundType::infer_from_name)
}

fn log_background_auto_rename_preparation(
    source: &SampleSource,
    request_count: usize,
    elapsed: std::time::Duration,
) {
    let elapsed_ms = elapsed.as_millis() as u64;
    if elapsed_ms >= 100 {
        warn!(
            source_id = %source.id,
            request_count,
            elapsed_ms,
            "auto rename: slow background request preparation"
        );
    } else {
        info!(
            source_id = %source.id,
            request_count,
            elapsed_ms,
            "auto rename: prepared background requests"
        );
    }
}
