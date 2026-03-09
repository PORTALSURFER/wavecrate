use super::helpers::fast_content_hash;
use super::*;
use crate::sample_sources::Rating;
use rusqlite::params;
use std::fs;
use std::time::SystemTime;

impl AppController {
    /// Register a newly exported clip in the browser and source database.
    ///
    /// When `looped` is true, the entry is flagged as a loop and any provided BPM is persisted.
    pub(crate) fn record_selection_entry(
        &mut self,
        request: SelectionEntryRecordRequest<'_>,
    ) -> Result<WavEntry, String> {
        let SelectionEntryRecordRequest {
            source,
            relative_path,
            target_tag,
            add_to_browser,
            register_in_source,
            looped,
            bpm,
        } = request;
        let metadata = fs::metadata(source.root.join(&relative_path))
            .map_err(|err| format!("Failed to read saved clip: {err}"))?;
        let modified_ns = metadata
            .modified()
            .map_err(|err| format!("Missing modified time for clip: {err}"))?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| "Clip modified time is before epoch".to_string())?
            .as_nanos() as i64;
        let entry = WavEntry {
            relative_path,
            file_size: metadata.len(),
            modified_ns,
            content_hash: None,
            tag: target_tag.unwrap_or(Rating::NEUTRAL),
            looped,
            missing: false,
            last_played_at: None,
        };
        if register_in_source {
            let db = self
                .database_for(source)
                .map_err(|err| format!("Database unavailable: {err}"))?;
            db.upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
                .map_err(|err| format!("Failed to register clip: {err}"))?;
            if entry.tag != Rating::NEUTRAL {
                db.set_tag(&entry.relative_path, entry.tag)
                    .map_err(|err| format!("Failed to tag clip: {err}"))?;
            }
            if looped {
                db.set_looped(&entry.relative_path, true)
                    .map_err(|err| format!("Failed to mark clip as looped: {err}"))?;
            }
            if let Some(bpm) = bpm {
                self.persist_selection_bpm(source, &entry, bpm)?;
            }
            if add_to_browser {
                if self.selection_state.ctx.selected_source.as_ref() == Some(&source.id)
                    && let Some(selected) = self.sample_view.wav.selected_wav.clone()
                {
                    self.runtime.jobs.set_pending_select_path(Some(selected));
                }
                self.insert_new_wav_entry(source);
            }
            self.enqueue_similarity_for_new_sample(
                source,
                &entry.relative_path,
                entry.file_size,
                entry.modified_ns,
            );
        }
        Ok(entry)
    }

    /// Invalidate browser caches after writing a new selection clip into a source.
    fn insert_new_wav_entry(&mut self, source: &SampleSource) {
        self.invalidate_wav_entries_for_source(source);
    }

    /// Persist BPM metadata for a looped exported clip in the analysis database.
    fn persist_selection_bpm(
        &self,
        source: &SampleSource,
        entry: &WavEntry,
        bpm: f32,
    ) -> Result<(), String> {
        if !bpm.is_finite() || bpm <= 0.0 {
            return Ok(());
        }
        let size = i64::try_from(entry.file_size)
            .map_err(|_| "Clip size exceeds database limits".to_string())?;
        let content_hash = fast_content_hash(entry.file_size, entry.modified_ns);
        let conn = analysis_jobs::open_source_db(&source.root)
            .map_err(|err| format!("Failed to open analysis database: {err}"))?;
        let sample_id = analysis_jobs::build_sample_id(source.id.as_str(), &entry.relative_path);
        conn.execute(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version, bpm)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, ?5)
             ON CONFLICT(sample_id) DO UPDATE SET
                 content_hash = excluded.content_hash,
                 size = excluded.size,
                 mtime_ns = excluded.mtime_ns,
                 bpm = excluded.bpm",
            params![
                sample_id,
                content_hash,
                size,
                entry.modified_ns,
                bpm as f64
            ],
        )
        .map_err(|err| format!("Failed to store clip BPM: {err}"))?;
        Ok(())
    }
}
