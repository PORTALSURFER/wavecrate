use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
};

use radiant::prelude as ui;

use crate::native_app::app::{
    ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmStage,
    WaveformCacheEntry,
};
use crate::native_app::waveform::{
    InstantWaveformPreview, PersistedPlaybackDescriptor, PreviewAuditionClip,
};

pub(in crate::native_app) struct WaveformCacheState {
    pub(in crate::native_app) entries: HashMap<PathBuf, WaveformCacheEntry>,
    pub(in crate::native_app) order: VecDeque<PathBuf>,
    pub(in crate::native_app) bytes: usize,
    pub(in crate::native_app) indicator_refresh_task: ui::LatestTask,
    pub(in crate::native_app) warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) warm_tasks: ui::ResourceTasks,
    pub(in crate::native_app) warm_key: Option<ui::ResourceKey>,
    pub(in crate::native_app) warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_plan_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_plan_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_delay_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_tasks: ui::ResourceTasks,
    pub(in crate::native_app) active_folder_warm_key: Option<ui::ResourceKey>,
    pub(in crate::native_app) active_folder_warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_folder_id: Option<String>,
    pub(in crate::native_app) active_folder_warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) active_folder_warm_completed: usize,
    pub(in crate::native_app) active_folder_warm_total: usize,
    pub(in crate::native_app) active_folder_warm_current: Option<PathBuf>,
    pub(in crate::native_app) active_folder_warm_current_progress: f32,
    pub(in crate::native_app) active_folder_warm_current_stage: Option<ActiveFolderCacheWarmStage>,
    pub(in crate::native_app) active_folder_warm_batch_base_completed: usize,
    pub(in crate::native_app) cached_sample_paths: HashSet<String>,
    pub(in crate::native_app) instant_audition_sample_paths: HashSet<String>,
    pub(in crate::native_app) instant_audition_descriptors:
        HashMap<PathBuf, PersistedPlaybackDescriptor>,
    preview_audition_clips: HashMap<PathBuf, PreviewAuditionCacheEntry>,
    instant_waveform_previews: HashMap<PathBuf, InstantWaveformPreviewCacheEntry>,
    preview_audition_sample_paths: HashSet<String>,
    preview_audition_bytes: usize,
    preview_audition_tick: u64,
    instant_waveform_preview_bytes: usize,
    instant_waveform_preview_tick: u64,
    preview_audition_attempted_paths: HashSet<String>,
    preview_audition_scheduled_paths: HashSet<String>,
    preview_audition_failed_paths: HashSet<String>,
    preview_audition_starmap_warm_signature: Option<u64>,
    preview_audition_starmap_warm_scheduled: usize,
    preview_audition_pending_starmap_warm_reservation: Option<PreviewAuditionWarmReservation>,
    preview_audition_list_warm_signature: Option<u64>,
    preview_audition_list_warm_scheduled: usize,
    preview_audition_pending_list_warm_reservation: Option<PreviewAuditionWarmReservation>,
}

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq)]
struct PreviewAuditionCacheEntry {
    clip: PreviewAuditionClip,
    byte_len: usize,
    last_used: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct InstantWaveformPreviewCacheEntry {
    preview: InstantWaveformPreview,
    byte_len: usize,
    last_used: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PreviewAuditionWarmReservation {
    signature: u64,
    count: usize,
}

const PREVIEW_AUDITION_CACHE_MAX_BYTES: usize = 96 * 1024 * 1024;
#[cfg(not(test))]
const INSTANT_WAVEFORM_PREVIEW_CACHE_MAX_BYTES: usize = 64 * 1024 * 1024;
#[cfg(test)]
const INSTANT_WAVEFORM_PREVIEW_CACHE_MAX_BYTES: usize = 1_024;

impl Default for WaveformCacheState {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: Default::default(),
            bytes: 0,
            indicator_refresh_task: ui::LatestTask::new(),
            warm_pending: Default::default(),
            warm_tasks: ui::ResourceTasks::new(),
            warm_key: None,
            warm_cancel: None,
            active_folder_warm_plan_task: ui::LatestTask::new(),
            active_folder_warm_plan_cancel: None,
            active_folder_warm_delay_task: ui::LatestTask::new(),
            active_folder_warm_tasks: ui::ResourceTasks::new(),
            active_folder_warm_key: None,
            active_folder_warm_cancel: None,
            active_folder_warm_folder_id: None,
            active_folder_warm_pending: Default::default(),
            active_folder_warm_completed: 0,
            active_folder_warm_total: 0,
            active_folder_warm_current: None,
            active_folder_warm_current_progress: 0.0,
            active_folder_warm_current_stage: None,
            active_folder_warm_batch_base_completed: 0,
            cached_sample_paths: Default::default(),
            instant_audition_sample_paths: Default::default(),
            instant_audition_descriptors: Default::default(),
            preview_audition_clips: Default::default(),
            instant_waveform_previews: Default::default(),
            preview_audition_sample_paths: Default::default(),
            preview_audition_bytes: 0,
            preview_audition_tick: 0,
            instant_waveform_preview_bytes: 0,
            instant_waveform_preview_tick: 0,
            preview_audition_attempted_paths: Default::default(),
            preview_audition_scheduled_paths: Default::default(),
            preview_audition_failed_paths: Default::default(),
            preview_audition_starmap_warm_signature: None,
            preview_audition_starmap_warm_scheduled: 0,
            preview_audition_pending_starmap_warm_reservation: None,
            preview_audition_list_warm_signature: None,
            preview_audition_list_warm_scheduled: 0,
            preview_audition_pending_list_warm_reservation: None,
        }
    }
}

impl WaveformCacheState {
    /// Evict every volatile path-owned cache record below a removed source root.
    ///
    /// This intentionally leaves persisted waveform payloads alone; durable cleanup uses the
    /// source database's reverse-ownership manifest after old lifecycle work has drained.
    pub(in crate::native_app) fn release_source_runtime(&mut self, root: &Path) -> usize {
        let entry_paths = self
            .entries
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect::<Vec<_>>();
        for path in &entry_paths {
            if let Some(entry) = self.entries.remove(path) {
                self.bytes = self.bytes.saturating_sub(entry.byte_len);
            }
        }
        let removed_entries = entry_paths.len();
        self.order.retain(|path| !path.starts_with(root));
        self.warm_pending.retain(|path| !path.starts_with(root));
        self.active_folder_warm_pending
            .retain(|path| !path.starts_with(root));
        if self
            .active_folder_warm_current
            .as_ref()
            .is_some_and(|path| path.starts_with(root))
        {
            self.clear_active_folder_warm_current();
        }
        self.cached_sample_paths
            .retain(|path| !Path::new(path).starts_with(root));
        self.instant_audition_sample_paths
            .retain(|path| !Path::new(path).starts_with(root));
        self.instant_audition_descriptors
            .retain(|path, _| !path.starts_with(root));

        let preview_paths = self
            .preview_audition_clips
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect::<Vec<_>>();
        for path in preview_paths {
            if let Some(entry) = self.preview_audition_clips.remove(&path) {
                self.preview_audition_bytes =
                    self.preview_audition_bytes.saturating_sub(entry.byte_len);
            }
        }
        let waveform_preview_paths = self
            .instant_waveform_previews
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect::<Vec<_>>();
        for path in waveform_preview_paths {
            if let Some(entry) = self.instant_waveform_previews.remove(&path) {
                self.instant_waveform_preview_bytes = self
                    .instant_waveform_preview_bytes
                    .saturating_sub(entry.byte_len);
            }
        }
        self.preview_audition_sample_paths
            .retain(|path| !Path::new(path).starts_with(root));
        self.preview_audition_attempted_paths
            .retain(|path| !Path::new(path).starts_with(root));
        let scheduled_before = self.preview_audition_scheduled_paths.len();
        self.preview_audition_scheduled_paths
            .retain(|path| !Path::new(path).starts_with(root));
        let removed_scheduled =
            scheduled_before.saturating_sub(self.preview_audition_scheduled_paths.len());
        self.release_pending_preview_warm_reservation(removed_scheduled);
        self.preview_audition_failed_paths
            .retain(|path| !Path::new(path).starts_with(root));
        removed_entries
    }

    pub(in crate::native_app) fn start_active_folder_warm_plan(
        &mut self,
        folder_id: String,
        total: usize,
    ) {
        self.active_folder_warm_folder_id = Some(folder_id);
        self.active_folder_warm_pending.clear();
        self.active_folder_warm_completed = 0;
        self.active_folder_warm_total = total;
        self.active_folder_warm_current = None;
        self.active_folder_warm_current_progress = 0.0;
        self.active_folder_warm_current_stage = Some(ActiveFolderCacheWarmStage::CheckingCache);
        self.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn start_active_folder_warm_decode_queue(
        &mut self,
        folder_id: String,
        pending: Vec<PathBuf>,
    ) {
        let total = pending.len();
        self.active_folder_warm_folder_id = Some(folder_id);
        self.active_folder_warm_pending = pending.into();
        self.active_folder_warm_completed = 0;
        self.active_folder_warm_total = total;
        self.clear_active_folder_warm_current();
        self.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn clear_active_folder_warm_job(&mut self) {
        self.active_folder_warm_folder_id = None;
        self.active_folder_warm_pending.clear();
        self.active_folder_warm_completed = 0;
        self.active_folder_warm_total = 0;
        self.clear_active_folder_warm_current();
        self.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn clear_active_folder_warm_current(&mut self) {
        self.active_folder_warm_current = None;
        self.active_folder_warm_current_progress = 0.0;
        self.active_folder_warm_current_stage = None;
    }

    pub(in crate::native_app) fn apply_active_folder_warm_plan_progress(
        &mut self,
        progress: ActiveFolderCacheWarmPlanProgress,
    ) {
        self.active_folder_warm_completed = progress.checked.min(progress.total);
        self.active_folder_warm_total = progress.total;
        self.active_folder_warm_current = Some(progress.path.clone());
        self.active_folder_warm_current_progress = if progress.total == 0 {
            1.0
        } else {
            progress.checked as f32 / progress.total as f32
        };
        self.active_folder_warm_current_stage = Some(ActiveFolderCacheWarmStage::CheckingCache);
        if progress.playback_ready {
            self.mark_sample_playback_cache_ready(&progress.path);
        }
    }

    pub(in crate::native_app) fn apply_active_folder_warm_progress(
        &mut self,
        progress: ActiveFolderCacheWarmProgress,
    ) {
        let cached_path = progress.cached.then(|| progress.path.clone());
        self.active_folder_warm_completed = self
            .active_folder_warm_batch_base_completed
            .saturating_add(progress.processed)
            .min(self.active_folder_warm_total);
        self.active_folder_warm_current = Some(progress.path);
        self.active_folder_warm_current_progress = progress.current_progress.clamp(0.0, 1.0);
        self.active_folder_warm_current_stage = Some(progress.stage);
        if let Some(path) = cached_path {
            self.mark_sample_playback_cache_ready(&path);
        }
    }

    pub(in crate::native_app) fn complete_active_folder_warm_batch(&mut self, processed: usize) {
        self.active_folder_warm_completed = self
            .active_folder_warm_batch_base_completed
            .saturating_add(processed)
            .min(self.active_folder_warm_total);
        self.clear_active_folder_warm_current();
    }

    pub(in crate::native_app) fn begin_active_folder_warm_batch(
        &mut self,
        first_path: Option<PathBuf>,
    ) {
        self.active_folder_warm_batch_base_completed = self.active_folder_warm_completed;
        self.active_folder_warm_current = first_path;
        self.active_folder_warm_current_progress = 0.0;
        self.active_folder_warm_current_stage = None;
    }

    pub(in crate::native_app) fn mark_sample_playback_cache_ready(&mut self, path: &Path) {
        let file_id = path.display().to_string();
        self.cached_sample_paths.insert(file_id.clone());
        self.instant_audition_sample_paths.insert(file_id);
    }

    pub(in crate::native_app) fn mark_sample_playback_descriptor_ready(
        &mut self,
        descriptor: PersistedPlaybackDescriptor,
    ) {
        let file_id = descriptor.path.display().to_string();
        self.cached_sample_paths.insert(file_id.clone());
        self.instant_audition_sample_paths.insert(file_id);
        self.instant_audition_descriptors
            .insert(descriptor.path.clone(), descriptor);
    }

    pub(in crate::native_app) fn clear_sample_instant_audition(&mut self, path: &Path) {
        let file_id = path.display().to_string();
        self.cached_sample_paths.remove(&file_id);
        self.instant_audition_sample_paths.remove(&file_id);
        self.instant_audition_descriptors.remove(path);
        if let Some(entry) = self.preview_audition_clips.remove(path) {
            self.preview_audition_bytes =
                self.preview_audition_bytes.saturating_sub(entry.byte_len);
        }
        if let Some(entry) = self.instant_waveform_previews.remove(path) {
            self.instant_waveform_preview_bytes = self
                .instant_waveform_preview_bytes
                .saturating_sub(entry.byte_len);
        }
        self.preview_audition_sample_paths.remove(&file_id);
        self.preview_audition_attempted_paths.remove(&file_id);
        self.preview_audition_scheduled_paths.remove(&file_id);
        self.preview_audition_failed_paths.remove(&file_id);
    }

    pub(in crate::native_app) fn preview_audition_clip(
        &mut self,
        path: &Path,
    ) -> Option<PreviewAuditionClip> {
        // Hot audition paths call this per target. Trust the in-memory preview
        // entry here; source refresh invalidation removes changed samples.
        let tick = self.next_preview_audition_tick();
        let entry = self.preview_audition_clips.get_mut(path)?;
        entry.last_used = tick;
        Some(entry.clip.clone())
    }

    pub(in crate::native_app) fn instant_waveform_preview(
        &mut self,
        path: &Path,
    ) -> Option<InstantWaveformPreview> {
        let tick = self.next_instant_waveform_preview_tick();
        let entry = self.instant_waveform_previews.get_mut(path)?;
        entry.last_used = tick;
        Some(entry.preview.clone())
    }

    pub(in crate::native_app) fn preview_audition_warm_needed(&self, path: &Path) -> bool {
        !self.preview_audition_clips.contains_key(path)
            && !self
                .preview_audition_attempted_paths
                .contains(&path.display().to_string())
            && !self
                .preview_audition_scheduled_paths
                .contains(&path.display().to_string())
    }

    pub(in crate::native_app) fn preview_audition_decode_needed(&self, path: &Path) -> bool {
        !self.preview_audition_clips.contains_key(path)
            && !self
                .preview_audition_failed_paths
                .contains(&path.display().to_string())
    }

    pub(in crate::native_app) fn preview_audition_sample_paths(&self) -> &HashSet<String> {
        &self.preview_audition_sample_paths
    }

    #[cfg(test)]
    pub(in crate::native_app) fn preview_audition_scheduled_paths(&self) -> &HashSet<String> {
        &self.preview_audition_scheduled_paths
    }

    pub(in crate::native_app) fn mark_preview_audition_failed(&mut self, path: &Path) {
        let file_id = path.display().to_string();
        self.preview_audition_attempted_paths
            .insert(file_id.clone());
        self.preview_audition_scheduled_paths.remove(&file_id);
        self.preview_audition_failed_paths.insert(file_id);
    }

    pub(in crate::native_app) fn mark_preview_audition_warm_scheduled(&mut self, paths: &[String]) {
        for path in paths {
            self.preview_audition_scheduled_paths.insert(path.clone());
        }
    }

    pub(in crate::native_app) fn remaining_starmap_preview_warm_budget(
        &mut self,
        signature: u64,
        budget: usize,
    ) -> usize {
        if self.preview_audition_starmap_warm_signature != Some(signature) {
            self.preview_audition_starmap_warm_signature = Some(signature);
            self.preview_audition_starmap_warm_scheduled = 0;
        }
        budget.saturating_sub(self.preview_audition_starmap_warm_scheduled)
    }

    pub(in crate::native_app) fn reserve_starmap_preview_warm_budget(
        &mut self,
        signature: u64,
        scheduled: usize,
    ) {
        if self.preview_audition_starmap_warm_signature != Some(signature) {
            self.preview_audition_starmap_warm_signature = Some(signature);
            self.preview_audition_starmap_warm_scheduled = 0;
        }
        self.preview_audition_starmap_warm_scheduled = self
            .preview_audition_starmap_warm_scheduled
            .saturating_add(scheduled);
    }

    pub(in crate::native_app) fn reserve_starmap_preview_warm_batch(
        &mut self,
        signature: u64,
        scheduled: usize,
    ) {
        self.reserve_starmap_preview_warm_budget(signature, scheduled);
        self.preview_audition_pending_starmap_warm_reservation =
            Some(PreviewAuditionWarmReservation {
                signature,
                count: scheduled,
            });
    }

    pub(in crate::native_app) fn remaining_list_preview_warm_budget(
        &mut self,
        signature: u64,
        budget: usize,
    ) -> usize {
        if self.preview_audition_list_warm_signature != Some(signature) {
            self.preview_audition_list_warm_signature = Some(signature);
            self.preview_audition_list_warm_scheduled = 0;
        }
        budget.saturating_sub(self.preview_audition_list_warm_scheduled)
    }

    pub(in crate::native_app) fn reserve_list_preview_warm_budget(
        &mut self,
        signature: u64,
        scheduled: usize,
    ) {
        if self.preview_audition_list_warm_signature != Some(signature) {
            self.preview_audition_list_warm_signature = Some(signature);
            self.preview_audition_list_warm_scheduled = 0;
        }
        self.preview_audition_list_warm_scheduled = self
            .preview_audition_list_warm_scheduled
            .saturating_add(scheduled);
    }

    pub(in crate::native_app) fn reserve_list_preview_warm_batch(
        &mut self,
        signature: u64,
        scheduled: usize,
    ) {
        self.reserve_list_preview_warm_budget(signature, scheduled);
        self.preview_audition_pending_list_warm_reservation =
            Some(PreviewAuditionWarmReservation {
                signature,
                count: scheduled,
            });
    }

    pub(in crate::native_app) fn finish_preview_audition_warm_schedule(
        &mut self,
        scheduled_paths: &[String],
        attempted_paths: &[String],
        failed_paths: &[String],
    ) {
        let attempted_count = attempted_paths.len().min(scheduled_paths.len());
        let unattempted_count = scheduled_paths.len().saturating_sub(attempted_count);
        for path in scheduled_paths {
            self.preview_audition_scheduled_paths.remove(path);
        }
        for path in attempted_paths {
            self.preview_audition_attempted_paths.insert(path.clone());
        }
        for path in failed_paths {
            self.preview_audition_attempted_paths.insert(path.clone());
            self.preview_audition_failed_paths.insert(path.clone());
        }
        self.release_pending_preview_warm_reservation(unattempted_count);
    }

    pub(in crate::native_app) fn cancel_preview_audition_warm_schedule(&mut self) {
        self.release_pending_preview_warm_reservation(self.preview_audition_scheduled_paths.len());
        self.preview_audition_scheduled_paths.clear();
    }

    fn release_pending_preview_warm_reservation(&mut self, count: usize) {
        if count == 0 {
            self.preview_audition_pending_starmap_warm_reservation = None;
            self.preview_audition_pending_list_warm_reservation = None;
            return;
        }
        if let Some(reservation) = self
            .preview_audition_pending_starmap_warm_reservation
            .take()
        {
            if self.preview_audition_starmap_warm_signature == Some(reservation.signature) {
                self.preview_audition_starmap_warm_scheduled = self
                    .preview_audition_starmap_warm_scheduled
                    .saturating_sub(count.min(reservation.count));
            }
        }
        if let Some(reservation) = self.preview_audition_pending_list_warm_reservation.take() {
            if self.preview_audition_list_warm_signature == Some(reservation.signature) {
                self.preview_audition_list_warm_scheduled = self
                    .preview_audition_list_warm_scheduled
                    .saturating_sub(count.min(reservation.count));
            }
        }
    }

    pub(in crate::native_app) fn store_preview_audition_clip(&mut self, clip: PreviewAuditionClip) {
        let path = clip.path.clone();
        let file_id = path.display().to_string();
        let byte_len = clip.byte_len();
        let last_used = self.next_preview_audition_tick();
        if let Some(previous) = self.preview_audition_clips.insert(
            path,
            PreviewAuditionCacheEntry {
                clip,
                byte_len,
                last_used,
            },
        ) {
            self.preview_audition_bytes = self
                .preview_audition_bytes
                .saturating_sub(previous.byte_len);
        }
        self.preview_audition_bytes = self.preview_audition_bytes.saturating_add(byte_len);
        self.preview_audition_sample_paths.insert(file_id.clone());
        self.preview_audition_attempted_paths
            .insert(file_id.clone());
        self.preview_audition_scheduled_paths.remove(&file_id);
        self.preview_audition_failed_paths.remove(&file_id);
        self.prune_preview_audition_cache();
    }

    pub(in crate::native_app) fn store_instant_waveform_preview(
        &mut self,
        preview: InstantWaveformPreview,
    ) {
        let path = preview.path().to_path_buf();
        let byte_len = preview.byte_len();
        let last_used = self.next_instant_waveform_preview_tick();
        if let Some(previous) = self.instant_waveform_previews.insert(
            path,
            InstantWaveformPreviewCacheEntry {
                preview,
                byte_len,
                last_used,
            },
        ) {
            self.instant_waveform_preview_bytes = self
                .instant_waveform_preview_bytes
                .saturating_sub(previous.byte_len);
        }
        self.instant_waveform_preview_bytes =
            self.instant_waveform_preview_bytes.saturating_add(byte_len);
        self.prune_instant_waveform_preview_cache();
    }

    fn evict_preview_audition_clip(&mut self, path: &Path) {
        let file_id = path.display().to_string();
        self.remove_preview_audition_clip_entry(path, &file_id);
    }

    fn remove_preview_audition_clip_entry(&mut self, path: &Path, file_id: &str) {
        if let Some(entry) = self.preview_audition_clips.remove(path) {
            self.preview_audition_bytes =
                self.preview_audition_bytes.saturating_sub(entry.byte_len);
        }
        self.preview_audition_sample_paths.remove(file_id);
        if !self.cached_sample_paths.contains(file_id)
            && !self.instant_audition_descriptors.contains_key(path)
        {
            self.instant_audition_sample_paths.remove(file_id);
        }
    }

    fn next_preview_audition_tick(&mut self) -> u64 {
        self.preview_audition_tick = self.preview_audition_tick.saturating_add(1);
        self.preview_audition_tick
    }

    fn next_instant_waveform_preview_tick(&mut self) -> u64 {
        self.instant_waveform_preview_tick = self.instant_waveform_preview_tick.saturating_add(1);
        self.instant_waveform_preview_tick
    }

    fn prune_preview_audition_cache(&mut self) {
        while self.preview_audition_bytes > PREVIEW_AUDITION_CACHE_MAX_BYTES {
            let Some(oldest_path) = self
                .preview_audition_clips
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(path, _)| path.clone())
            else {
                self.preview_audition_bytes = 0;
                return;
            };
            self.evict_preview_audition_clip(&oldest_path);
        }
    }

    fn prune_instant_waveform_preview_cache(&mut self) {
        while self.instant_waveform_preview_bytes > INSTANT_WAVEFORM_PREVIEW_CACHE_MAX_BYTES {
            let Some(oldest_path) = self
                .instant_waveform_previews
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(path, _)| path.clone())
            else {
                self.instant_waveform_preview_bytes = 0;
                return;
            };
            if let Some(entry) = self.instant_waveform_previews.remove(&oldest_path) {
                self.instant_waveform_preview_bytes = self
                    .instant_waveform_preview_bytes
                    .saturating_sub(entry.byte_len);
            }
        }
    }
}
