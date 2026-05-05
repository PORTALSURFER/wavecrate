use super::*;

#[derive(Clone)]
pub(super) struct AutoRenameBackgroundRequest {
    pub(super) source: SampleSource,
    pub(super) paths: Vec<PathBuf>,
    pub(super) identifier: String,
    pub(super) is_playing: bool,
    pub(super) resume_looped: bool,
    pub(super) resume_start_override: Option<f64>,
    pub(super) loaded_relative: Option<PathBuf>,
    pub(super) metadata: HashMap<PathBuf, AutoRenamePathMetadata>,
}

#[derive(Clone, Default)]
pub(super) struct AutoRenamePathMetadata {
    pub(super) entry: Option<WavEntry>,
    pub(super) normal_tags: Option<Vec<String>>,
    pub(super) bpm: Option<f32>,
}

impl BrowserController<'_> {
    pub(super) fn capture_auto_rename_background_request(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
    ) -> AutoRenameBackgroundRequest {
        let loaded_relative = self
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .filter(|audio| audio.source_id == source.id)
            .map(|audio| audio.relative_path.clone());
        let mut metadata = HashMap::new();
        for relative_path in paths {
            let entry = self.cached_auto_rename_entry(source, relative_path);
            let normal_tags = self
                .ui_cache
                .browser
                .normal_tags
                .get(&source.id)
                .and_then(|source_tags| source_tags.get(relative_path))
                .map(|tags| {
                    tags.iter()
                        .map(|tag| tag.display_label.clone())
                        .collect::<Vec<_>>()
                })
                .or_else(|| entry.as_ref().map(|entry| entry.normal_tags.clone()));
            let bpm = self
                .ui_cache
                .browser
                .bpm_values
                .get(&source.id)
                .and_then(|source_bpms| source_bpms.get(relative_path))
                .copied()
                .flatten();
            if entry.is_some() || normal_tags.is_some() || bpm.is_some() {
                metadata.insert(
                    relative_path.clone(),
                    AutoRenamePathMetadata {
                        entry,
                        normal_tags,
                        bpm,
                    },
                );
            }
        }
        AutoRenameBackgroundRequest {
            source: source.clone(),
            paths: paths.to_vec(),
            identifier: self.settings.default_identifier.clone(),
            is_playing: self.is_playing(),
            resume_looped: self.ui.waveform.loop_enabled,
            resume_start_override: self
                .ui
                .waveform
                .playhead
                .position
                .is_finite()
                .then(|| f64::from(self.ui.waveform.playhead.position.clamp(0.0, 1.0))),
            loaded_relative,
            metadata,
        }
    }

    fn cached_auto_rename_entry(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<WavEntry> {
        self.wav_index_for_path(relative_path)
            .and_then(|index| {
                let _ = self.ensure_wav_page_loaded(index);
                self.wav_entry(index).cloned()
            })
            .or_else(|| {
                self.cache
                    .wav
                    .entries
                    .get(&source.id)
                    .and_then(|cache| cache.lookup.get(relative_path).copied())
                    .and_then(|index| self.cache.wav.entries.get(&source.id)?.entry(index))
                    .cloned()
            })
    }

    /// Build auto-rename requests without taking a source-db write lock on the
    /// controller thread.
    pub(crate) fn prepare_auto_rename_requests(
        &mut self,
        source: &SampleSource,
        paths: &[PathBuf],
    ) -> Result<Vec<SampleAutoRenameRequest>, String> {
        let started_at = Instant::now();
        let db = crate::sample_sources::SourceDatabase::open_read_only(&source.root)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let mut requests = Vec::with_capacity(paths.len());
        let mut reserved_targets = HashSet::new();
        let identifier = self.settings.default_identifier.clone();
        let playhead_position = self.ui.waveform.playhead.position;
        let is_playing = self.is_playing();
        let resume_looped = self.ui.waveform.loop_enabled;
        for relative_path in paths {
            let tag = self.sample_tag_for(source, relative_path)?;
            let looped = self.sample_looped_for(source, relative_path)?;
            let locked = self.locked_for_path(source, relative_path)?;
            let last_played_at = self.sample_last_played_for(source, relative_path)?;
            let sound_type = self
                .live_sound_type_for_path(source, relative_path)
                .or(db
                    .sound_type_for_path(relative_path)
                    .map_err(|err| format!("Failed to read sound type: {err}"))?)
                .or_else(|| {
                    relative_path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .and_then(crate::sample_sources::SampleSoundType::infer_from_name)
                });
            let user_tag = self.live_user_tag_for_path(source, relative_path).or(db
                .user_tag_for_path(relative_path)
                .map_err(|err| format!("Failed to read custom tag: {err}"))?);
            let normal_tags = self.normal_tags_for_path(source, relative_path)?;
            let stem = build_auto_rename_stem(&AutoRenameInput {
                identifier: identifier.clone(),
                looped,
                sound_type,
                user_tag: user_tag.clone(),
                normal_tags: normal_tags
                    .iter()
                    .map(|tag| tag.display_label.clone())
                    .collect(),
                bpm: self.bpm_value_for_path(relative_path),
            });
            let new_relative = self.resolve_auto_rename_target(
                &source.root,
                relative_path,
                stem.tagged_basename.as_deref(),
                &stem.fallback_identifier,
                &mut reserved_targets,
            )?;
            let is_currently_loaded =
                self.sample_view
                    .wav
                    .loaded_audio
                    .as_ref()
                    .is_some_and(|audio| {
                        audio.source_id == source.id && audio.relative_path == *relative_path
                    });
            requests.push(SampleAutoRenameRequest {
                old_relative: relative_path.clone(),
                new_relative,
                tag,
                looped,
                locked,
                sound_type,
                user_tag,
                tag_named: stem.tag_based,
                last_played_at,
                resume_playback: is_currently_loaded && is_playing,
                resume_looped,
                resume_start_override: playhead_position
                    .is_finite()
                    .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
            });
        }
        let elapsed = started_at.elapsed();
        #[cfg(test)]
        crate::app::controller::batch_latency::record(
            crate::app::controller::batch_latency::BatchLatencySample::new(
                crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenamePrepare,
                requests.len(),
                elapsed,
            ),
        );
        log_prepared_auto_rename_requests(source, &requests, elapsed, "controller");
        self.log_auto_rename_preparation(source, requests.len(), elapsed);
        Ok(requests)
    }

    fn log_auto_rename_preparation(
        &self,
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
                "auto rename: slow controller request preparation"
            );
        } else {
            info!(
                source_id = %source.id,
                request_count,
                elapsed_ms,
                "auto rename: prepared controller requests"
            );
        }
    }

    pub(super) fn resolve_auto_rename_target(
        &self,
        root: &Path,
        relative_path: &Path,
        tagged_basename: Option<&str>,
        fallback_identifier: &str,
        reserved_targets: &mut HashSet<PathBuf>,
    ) -> Result<PathBuf, String> {
        if let Some(tagged_basename) = tagged_basename {
            if let Some(path) =
                self.try_auto_rename_target(root, relative_path, tagged_basename, reserved_targets)?
            {
                reserved_targets.insert(path.clone());
                return Ok(path);
            }
            for index in 1..=999 {
                let suffixed_basename = format!("{tagged_basename}_{index:03}");
                if let Some(path) = self.try_auto_rename_target(
                    root,
                    relative_path,
                    &suffixed_basename,
                    reserved_targets,
                )? {
                    reserved_targets.insert(path.clone());
                    return Ok(path);
                }
            }
        }
        for index in 1..=999 {
            let fallback_basename = format!("{fallback_identifier}_{index:03}");
            if let Some(path) = self.try_auto_rename_target(
                root,
                relative_path,
                &fallback_basename,
                reserved_targets,
            )? {
                reserved_targets.insert(path.clone());
                return Ok(path);
            }
        }
        Err(format!(
            "Unable to find a unique auto-rename target for {}",
            relative_path.display()
        ))
    }

    fn try_auto_rename_target(
        &self,
        root: &Path,
        relative_path: &Path,
        basename: &str,
        reserved_targets: &HashSet<PathBuf>,
    ) -> Result<Option<PathBuf>, String> {
        let full_name = self.name_with_preserved_extension(relative_path, basename)?;
        let new_relative = self.validate_new_sample_name_in_parent(relative_path, root, &full_name);
        match new_relative {
            Ok(path) if path == relative_path || !reserved_targets.contains(&path) => {
                Ok(Some(path))
            }
            Ok(_) => Ok(None),
            Err(err) if err.contains("already exists") => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn live_sound_type_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<crate::sample_sources::SampleSoundType> {
        self.wav_index_for_path(relative_path)
            .and_then(|index| {
                let _ = self.ensure_wav_page_loaded(index);
                self.wav_entry(index).and_then(|entry| entry.sound_type)
            })
            .or_else(|| {
                self.cache
                    .wav
                    .entries
                    .get(&source.id)
                    .and_then(|cache| cache.lookup.get(relative_path).copied())
                    .and_then(|index| self.cache.wav.entries.get(&source.id)?.entry(index))
                    .and_then(|entry| entry.sound_type)
            })
    }

    fn live_user_tag_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Option<String> {
        self.wav_index_for_path(relative_path)
            .and_then(|index| {
                let _ = self.ensure_wav_page_loaded(index);
                self.wav_entry(index)
                    .and_then(|entry| entry.user_tag.clone())
            })
            .or_else(|| {
                self.cache
                    .wav
                    .entries
                    .get(&source.id)
                    .and_then(|cache| cache.lookup.get(relative_path).copied())
                    .and_then(|index| self.cache.wav.entries.get(&source.id)?.entry(index))
                    .and_then(|entry| entry.user_tag.clone())
            })
    }

    fn locked_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
    ) -> Result<bool, String> {
        self.wav_index_for_path(relative_path)
            .and_then(|index| {
                let _ = self.ensure_wav_page_loaded(index);
                self.wav_entry(index).map(|entry| entry.locked)
            })
            .or_else(|| {
                self.cache
                    .wav
                    .entries
                    .get(&source.id)
                    .and_then(|cache| cache.lookup.get(relative_path).copied())
                    .and_then(|index| self.cache.wav.entries.get(&source.id)?.entry(index))
                    .map(|entry| entry.locked)
            })
            .or(self
                .database_for(source)
                .map_err(|err| err.to_string())?
                .locked_for_path(relative_path)
                .map_err(|err| err.to_string())?)
            .ok_or_else(|| format!("Sample not found: {}", relative_path.display()))
    }
}

pub(super) fn run_background_auto_rename_request(
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
        let overrides = snapshot.metadata.get(relative_path);
        let db_entry = match overrides.and_then(|metadata| metadata.entry.clone()) {
            Some(entry) => entry,
            None => db
                .entry_for_path(relative_path)
                .map_err(|err| format!("Failed to read sample metadata: {err}"))?
                .ok_or_else(|| format!("Sample not found: {}", relative_path.display()))?,
        };
        let normal_tags = match overrides.and_then(|metadata| metadata.normal_tags.clone()) {
            Some(tags) => tags,
            None => db
                .tags_for_path(relative_path)
                .map_err(|err| format!("Failed to read normal tags: {err}"))?
                .into_iter()
                .map(|tag| tag.display_label)
                .collect(),
        };
        let sound_type = db_entry.sound_type.or_else(|| {
            relative_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .and_then(crate::sample_sources::SampleSoundType::infer_from_name)
        });
        let bpm = overrides
            .and_then(|metadata| metadata.bpm)
            .or_else(|| bpm_lookup.get(sample_id.as_str()).copied().flatten());
        let stem = build_auto_rename_stem(&AutoRenameInput {
            identifier: snapshot.identifier.clone(),
            looped: db_entry.looped,
            sound_type,
            user_tag: db_entry.user_tag.clone(),
            normal_tags,
            bpm,
        });
        let new_relative = resolve_auto_rename_target_for_worker(
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
    #[cfg(test)]
    crate::app::controller::batch_latency::record(
        crate::app::controller::batch_latency::BatchLatencySample::new(
            crate::app::controller::batch_latency::BatchLatencyPhase::AutoRenamePrepare,
            requests.len(),
            elapsed,
        ),
    );
    log_background_auto_rename_preparation(&snapshot.source, requests.len(), elapsed);
    log_prepared_auto_rename_requests(&snapshot.source, &requests, elapsed, "background");
    Ok(requests)
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

fn log_prepared_auto_rename_requests(
    source: &SampleSource,
    requests: &[SampleAutoRenameRequest],
    elapsed: std::time::Duration,
    lane: &'static str,
) {
    info!(
        source_id = %source.id,
        lane,
        request_count = requests.len(),
        elapsed_ms = elapsed.as_millis() as u64,
        requests = %format_auto_rename_request_provenance(requests),
        "auto rename: request metadata provenance"
    );
}

fn format_auto_rename_request_provenance(requests: &[SampleAutoRenameRequest]) -> String {
    const MAX_ITEMS: usize = 8;
    let mut parts = requests
        .iter()
        .take(MAX_ITEMS)
        .map(|request| {
            format!(
                "{} -> {} looped={}",
                request.old_relative.display(),
                request.new_relative.display(),
                request.looped
            )
        })
        .collect::<Vec<_>>();
    if requests.len() > MAX_ITEMS {
        parts.push(format!("... +{} more", requests.len() - MAX_ITEMS));
    }
    parts.join("; ")
}

fn resolve_auto_rename_target_for_worker(
    root: &Path,
    relative_path: &Path,
    tagged_basename: Option<&str>,
    fallback_identifier: &str,
    reserved_targets: &mut HashSet<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(tagged_basename) = tagged_basename {
        if let Some(path) = try_auto_rename_target_for_worker(
            root,
            relative_path,
            tagged_basename,
            reserved_targets,
        )? {
            reserved_targets.insert(path.clone());
            return Ok(path);
        }
        for index in 1..=999 {
            let suffixed_basename = format!("{tagged_basename}_{index:03}");
            if let Some(path) = try_auto_rename_target_for_worker(
                root,
                relative_path,
                &suffixed_basename,
                reserved_targets,
            )? {
                reserved_targets.insert(path.clone());
                return Ok(path);
            }
        }
    }
    for index in 1..=999 {
        let fallback_basename = format!("{fallback_identifier}_{index:03}");
        if let Some(path) = try_auto_rename_target_for_worker(
            root,
            relative_path,
            &fallback_basename,
            reserved_targets,
        )? {
            reserved_targets.insert(path.clone());
            return Ok(path);
        }
    }
    Err(format!(
        "Unable to find a unique auto-rename target for {}",
        relative_path.display()
    ))
}

fn try_auto_rename_target_for_worker(
    root: &Path,
    relative_path: &Path,
    basename: &str,
    reserved_targets: &HashSet<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let full_name = name_with_preserved_extension_for_worker(relative_path, basename)?;
    let new_relative =
        validate_new_sample_name_in_parent_for_worker(relative_path, root, &full_name);
    match new_relative {
        Ok(path) if path == relative_path || !reserved_targets.contains(&path) => Ok(Some(path)),
        Ok(_) => Ok(None),
        Err(err) if err.contains("already exists") => Ok(None),
        Err(err) => Err(err),
    }
}

fn name_with_preserved_extension_for_worker(
    current_relative: &Path,
    new_name: &str,
) -> Result<String, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    let Some(ext) = current_relative.extension().and_then(|ext| ext.to_str()) else {
        return Ok(trimmed.to_string());
    };
    let ext_lower = ext.to_ascii_lowercase();
    let should_strip_suffix = |suffix: &str| -> bool {
        let suffix_lower = suffix.to_ascii_lowercase();
        suffix_lower == ext_lower
            || matches!(
                suffix_lower.as_str(),
                "wav" | "wave" | "flac" | "aif" | "aiff" | "mp3" | "ogg" | "opus"
            )
    };
    let stem = if let Some((stem, suffix)) = trimmed.rsplit_once('.') {
        if !stem.is_empty() && should_strip_suffix(suffix) {
            stem
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    let stem = stem.trim_end_matches('.');
    if stem.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    Ok(format!("{stem}.{ext}"))
}

fn validate_new_sample_name_in_parent_for_worker(
    relative_path: &Path,
    root: &Path,
    new_name: &str,
) -> Result<PathBuf, String> {
    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if trimmed.contains(['/', '\\']) {
        return Err("Name cannot contain path separators".into());
    }
    let parent = relative_path.parent().unwrap_or(Path::new(""));
    let new_relative = parent.join(trimmed);
    let new_absolute = root.join(&new_relative);
    if new_absolute.exists() && new_relative != relative_path {
        return Err(format!(
            "A file named {} already exists",
            new_relative.display()
        ));
    }
    Ok(new_relative)
}
