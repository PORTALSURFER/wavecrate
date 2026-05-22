use super::*;

impl BrowserController<'_> {
    pub(in crate::app::controller::library::browser_controller::actions::metadata) fn capture_auto_rename_background_request(
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
            let normal_tags = self.cached_auto_rename_normal_tags(source, relative_path, &entry);
            let bpm = self.cached_auto_rename_bpm(source, relative_path);
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

    fn cached_auto_rename_normal_tags(
        &self,
        source: &SampleSource,
        relative_path: &Path,
        entry: &Option<WavEntry>,
    ) -> Option<Vec<String>> {
        self.ui_cache
            .browser
            .normal_tags
            .get(&source.id)
            .and_then(|source_tags| source_tags.get(relative_path))
            .map(|tags| {
                tags.iter()
                    .map(|tag| tag.display_label.clone())
                    .collect::<Vec<_>>()
            })
            .or_else(|| entry.as_ref().map(|entry| entry.normal_tags.clone()))
    }

    fn cached_auto_rename_bpm(&self, source: &SampleSource, relative_path: &Path) -> Option<f32> {
        self.ui_cache
            .browser
            .bpm_values
            .get(&source.id)
            .and_then(|source_bpms| source_bpms.get(relative_path))
            .copied()
            .flatten()
    }
}
