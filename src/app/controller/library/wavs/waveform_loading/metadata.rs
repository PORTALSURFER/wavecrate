use super::*;

impl AppController {
    /// Invalidate in-memory and persisted waveform caches for one sample path.
    pub(crate) fn invalidate_cached_audio(&mut self, source_id: &SourceId, relative_path: &Path) {
        let key = CacheKey::new(source_id, relative_path);
        self.audio.cache.invalidate(&key);
        self.invalidate_persistent_waveform_cache(source_id, relative_path);
    }

    /// Queue source-analysis duration metadata persistence outside the load hot path.
    pub(super) fn defer_loaded_sample_duration_metadata_write(
        &mut self,
        source: &SampleSource,
        relative_path: &Path,
        duration_seconds: f32,
        sample_rate: u32,
    ) {
        if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
            return;
        }
        self.update_cached_duration_for_path(
            &source.id,
            relative_path,
            duration_seconds,
            sample_rate,
        );
        let long_sample_mark = (self.sample_view.wav.selected_wav.as_deref()
            == Some(relative_path))
        .then_some(duration_seconds > self.long_sample_threshold_seconds());
        if let Some(long_sample_mark) = long_sample_mark {
            self.update_cached_long_mark_for_path(&source.id, relative_path, long_sample_mark);
        }
        self.runtime.pending_loaded_duration_metadata = Some(
            crate::app::controller::state::runtime::PendingLoadedDurationMetadata {
                source_id: source.id.clone(),
                source_root: source.root.clone(),
                relative_path: relative_path.to_path_buf(),
                duration_seconds,
                sample_rate,
                long_sample_mark,
            },
        );
        self.runtime.pending_loaded_duration_metadata_not_before =
            Some(Instant::now() + LOADED_DURATION_METADATA_DEBOUNCE);
    }

    /// Return true when deferred loaded-duration metadata persistence is queued.
    pub(crate) fn has_pending_loaded_duration_metadata_write(&self) -> bool {
        self.runtime.pending_loaded_duration_metadata.is_some()
    }

    /// Flush deferred loaded-duration metadata persistence after debounce.
    pub(crate) fn flush_pending_loaded_duration_metadata_write(&mut self) {
        if self
            .runtime
            .pending_loaded_duration_metadata_not_before
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return;
        }
        self.runtime.pending_loaded_duration_metadata_not_before = None;
        let Some(pending) = self.runtime.pending_loaded_duration_metadata.take() else {
            return;
        };
        self.persist_loaded_duration_metadata(pending);
    }

    /// Persist deferred loaded-duration metadata to the source analysis database.
    fn persist_loaded_duration_metadata(
        &mut self,
        pending: crate::app::controller::state::runtime::PendingLoadedDurationMetadata,
    ) {
        let source = SampleSource {
            id: pending.source_id.clone(),
            root: pending.source_root.clone(),
        };
        let metadata = match self.current_file_metadata(&source, &pending.relative_path) {
            Ok(metadata) => metadata,
            Err(err) => {
                tracing::warn!(
                    "Failed to read file metadata for deferred duration update ({}): {err}",
                    pending.relative_path.display()
                );
                return;
            }
        };
        let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), &pending.relative_path);
        let content_hash =
            analysis_jobs::fast_content_hash(metadata.file_size, metadata.modified_ns);
        let mut conn = match analysis_jobs::open_source_db(&source.root) {
            Ok(conn) => conn,
            Err(err) => {
                tracing::warn!(
                    "Failed to open source DB for deferred duration update ({}): {err}",
                    pending.relative_path.display()
                );
                return;
            }
        };
        if let Err(err) = analysis_jobs::upsert_samples(
            &mut conn,
            &[analysis_jobs::SampleMetadata {
                sample_id: sample_id.clone(),
                content_hash,
                size: metadata.file_size,
                mtime_ns: metadata.modified_ns,
            }],
        ) {
            tracing::warn!(
                "Failed to ensure analysis row for {}: {err}",
                pending.relative_path.display()
            );
        }
        if let Err(err) = analysis_jobs::update_sample_duration(
            &conn,
            &sample_id,
            pending.duration_seconds,
            pending.sample_rate,
        ) {
            tracing::warn!(
                "Failed to store duration metadata for {}: {err}",
                pending.relative_path.display()
            );
        }
        if let Some(long_sample_mark) = pending.long_sample_mark
            && let Err(err) =
                analysis_jobs::update_sample_long_mark(&conn, &sample_id, long_sample_mark)
        {
            tracing::warn!(
                "Failed to store long sample mark for {}: {err}",
                pending.relative_path.display()
            );
        }
    }
}
