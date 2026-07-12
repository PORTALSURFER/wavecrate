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
        let db = crate::sample_sources::SourceDatabase::open_for_ui_read(&source.root)
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
            let sound_type = self.controller_sound_type_for_path(source, relative_path, &db)?;
            let user_tag = self.controller_user_tag_for_path(source, relative_path, &db)?;
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
                resume_playback: self.is_auto_rename_sample_loaded(source, relative_path)
                    && is_playing,
                resume_looped,
                resume_start_override: playhead_position
                    .is_finite()
                    .then(|| f64::from(playhead_position.clamp(0.0, 1.0))),
            });
        }
        logging::record_auto_rename_prepare_latency(requests.len(), started_at.elapsed());
        logging::log_prepared_auto_rename_requests(
            source,
            &requests,
            started_at.elapsed(),
            "controller",
        );
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

    fn controller_sound_type_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        db: &crate::sample_sources::SourceDatabase,
    ) -> Result<Option<crate::sample_sources::SampleSoundType>, String> {
        let db_sound_type = db
            .sound_type_for_path(relative_path)
            .map_err(|err| format!("Failed to read sound type: {err}"))?;
        Ok(self
            .live_sound_type_for_path(source, relative_path)
            .or(db_sound_type)
            .or_else(|| infer_sound_type_from_path(relative_path)))
    }

    fn controller_user_tag_for_path(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        db: &crate::sample_sources::SourceDatabase,
    ) -> Result<Option<String>, String> {
        let db_user_tag = db
            .user_tag_for_path(relative_path)
            .map_err(|err| format!("Failed to read custom tag: {err}"))?;
        Ok(self
            .live_user_tag_for_path(source, relative_path)
            .or(db_user_tag))
    }

    fn is_auto_rename_sample_loaded(&self, source: &SampleSource, relative_path: &Path) -> bool {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .is_some_and(|audio| {
                audio.source_id == source.id && audio.relative_path == relative_path
            })
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

fn infer_sound_type_from_path(
    relative_path: &Path,
) -> Option<crate::sample_sources::SampleSoundType> {
    relative_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(crate::sample_sources::SampleSoundType::infer_from_name)
}
