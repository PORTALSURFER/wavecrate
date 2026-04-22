use super::*;

impl BrowserController<'_> {
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
            let stem = build_auto_rename_stem(&AutoRenameInput {
                identifier: identifier.clone(),
                looped,
                sound_type,
                user_tag,
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
                sound_type,
                resume_playback: is_currently_loaded && is_playing,
                resume_looped,
                resume_start_override: playhead_position
                    .is_finite()
                    .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
            });
        }
        self.log_auto_rename_preparation(source, requests.len(), started_at.elapsed());
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
}
