use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
};

use radiant::prelude as ui;

use crate::native_app::app::{
    ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmStage,
    SampleSelectionLoadState, WaveformCacheEntry,
};
use crate::native_app::waveform::{
    PersistedPlaybackDescriptor, PreviewAuditionClip, WaveformState,
};
use wavecrate::selection::SelectionRange;

pub(in crate::native_app) struct WaveformAppState {
    pub(in crate::native_app) current: WaveformState,
    pub(in crate::native_app) load: WaveformLoadState,
    pub(in crate::native_app) cache: WaveformCacheState,
    pub(in crate::native_app) pending_play_selection_transaction:
        Option<WaveformPlaySelectionSnapshot>,
    pub(in crate::native_app) pending_edit_fade_transaction: Option<WaveformEditSelectionSnapshot>,
    pub(in crate::native_app) pending_edit_selection_transaction:
        Option<WaveformEditSelectionSnapshot>,
    pub(in crate::native_app) pending_play_selection_retarget: bool,
    pub(in crate::native_app) pending_play_selection_retarget_cycle:
        Option<PendingPlaySelectionRetargetCycle>,
}

impl WaveformAppState {
    pub(in crate::native_app) fn new(current: WaveformState) -> Self {
        Self {
            current,
            load: WaveformLoadState::default(),
            cache: WaveformCacheState::default(),
            pending_play_selection_transaction: None,
            pending_edit_fade_transaction: None,
            pending_edit_selection_transaction: None,
            pending_play_selection_retarget: false,
            pending_play_selection_retarget_cycle: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PendingPlaySelectionRetargetCycle {
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) last_progress_ratio: Option<f32>,
}

impl PendingPlaySelectionRetargetCycle {
    pub(in crate::native_app) fn new(end_ratio: f32, last_progress_ratio: Option<f32>) -> Self {
        Self {
            end_ratio,
            last_progress_ratio,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformEditSelectionSnapshot {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) edit_selection: Option<SelectionRange>,
}

impl WaveformEditSelectionSnapshot {
    pub(in crate::native_app) fn from_waveform(waveform: &WaveformState) -> Self {
        Self {
            path: waveform.path(),
            edit_selection: waveform.edit_selection(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformPlaySelectionSnapshot {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) play_mark_ratio: Option<f32>,
    pub(in crate::native_app) play_selection: Option<SelectionRange>,
    pub(in crate::native_app) marked_play_ranges: Vec<SelectionRange>,
}

impl WaveformPlaySelectionSnapshot {
    pub(in crate::native_app) fn from_waveform(waveform: &WaveformState) -> Self {
        Self {
            path: waveform.path(),
            play_mark_ratio: waveform.play_mark_ratio(),
            play_selection: waveform.play_selection(),
            marked_play_ranges: waveform.marked_play_ranges().to_vec(),
        }
    }
}

pub(in crate::native_app) struct WaveformLoadState {
    pub(in crate::native_app) progress: f32,
    pub(in crate::native_app) target_progress: f32,
    pub(in crate::native_app) label: Option<String>,
    pub(in crate::native_app) selection: SampleSelectionLoadState,
}

impl Default for WaveformLoadState {
    fn default() -> Self {
        Self {
            progress: 0.0,
            target_progress: 0.0,
            label: None,
            selection: SampleSelectionLoadState::default(),
        }
    }
}

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
    preview_audition_sample_paths: HashSet<String>,
    preview_audition_bytes: usize,
    preview_audition_tick: u64,
    preview_audition_attempted_paths: HashSet<String>,
    preview_audition_scheduled_paths: HashSet<String>,
    preview_audition_failed_paths: HashSet<String>,
    preview_audition_starmap_warm_signature: Option<u64>,
    preview_audition_starmap_warm_scheduled: usize,
    preview_audition_list_warm_signature: Option<u64>,
    preview_audition_list_warm_scheduled: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct PreviewAuditionCacheEntry {
    clip: PreviewAuditionClip,
    byte_len: usize,
    last_used: u64,
}

const PREVIEW_AUDITION_CACHE_MAX_BYTES: usize = 96 * 1024 * 1024;

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
            preview_audition_sample_paths: Default::default(),
            preview_audition_bytes: 0,
            preview_audition_tick: 0,
            preview_audition_attempted_paths: Default::default(),
            preview_audition_scheduled_paths: Default::default(),
            preview_audition_failed_paths: Default::default(),
            preview_audition_starmap_warm_signature: None,
            preview_audition_starmap_warm_scheduled: 0,
            preview_audition_list_warm_signature: None,
            preview_audition_list_warm_scheduled: 0,
        }
    }
}

impl WaveformCacheState {
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
            self.preview_audition_attempted_paths.insert(path.clone());
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

    pub(in crate::native_app) fn finish_preview_audition_warm_schedule(
        &mut self,
        scheduled_paths: &[String],
        attempted_paths: &[String],
        failed_paths: &[String],
    ) {
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
    }

    pub(in crate::native_app) fn cancel_preview_audition_warm_schedule(&mut self) {
        self.preview_audition_scheduled_paths.clear();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc, time::SystemTime};

    fn preview_clip(path: PathBuf) -> PreviewAuditionClip {
        PreviewAuditionClip {
            path,
            source_len: 0,
            source_modified: Some(SystemTime::UNIX_EPOCH),
            samples: Arc::from([0.0_f32]),
            sample_rate: 44_100,
            channels: 1,
            frames: 1,
            normalized_gain: 1.0,
        }
    }

    #[test]
    fn preview_warm_attempt_marker_survives_missing_clip_probe() {
        let path = PathBuf::from("/tmp/wavecrate-preview-missing.wav");
        let path_id = path.display().to_string();
        let mut cache = WaveformCacheState::default();

        assert!(cache.preview_audition_warm_needed(&path));
        cache.finish_preview_audition_warm_schedule(
            std::slice::from_ref(&path_id),
            std::slice::from_ref(&path_id),
            std::slice::from_ref(&path_id),
        );
        assert!(!cache.preview_audition_warm_needed(&path));
        assert_eq!(cache.preview_audition_clip(&path), None);
        assert!(
            !cache.preview_audition_warm_needed(&path),
            "a failed warm attempt must not become eligible again on the next frame"
        );
    }

    #[test]
    fn preview_clip_lookup_trusts_cached_head_without_file_metadata_probe() {
        let path = PathBuf::from("/tmp/wavecrate-preview-head-without-source-file.wav");
        let mut cache = WaveformCacheState::default();
        cache.store_preview_audition_clip(preview_clip(path.clone()));

        let clip = cache.preview_audition_clip(&path);

        assert!(
            clip.is_some(),
            "hot preview-head playback lookup should not require a synchronous filesystem metadata probe"
        );
        assert!(
            cache.preview_audition_clips.contains_key(&path),
            "hot lookup should not evict a cached preview head just because the source file is unavailable during the UI update"
        );
    }

    #[test]
    fn preview_warm_scheduled_path_is_not_requeued_before_completion() {
        let path = PathBuf::from("/tmp/wavecrate-preview-scheduled.wav");
        let path_id = path.display().to_string();
        let mut cache = WaveformCacheState::default();

        assert!(cache.preview_audition_warm_needed(&path));
        cache.mark_preview_audition_warm_scheduled(std::slice::from_ref(&path_id));
        assert!(
            !cache.preview_audition_warm_needed(&path),
            "a scheduled warm path must not be rediscovered by the next frame"
        );

        cache.finish_preview_audition_warm_schedule(
            std::slice::from_ref(&path_id),
            std::slice::from_ref(&path_id),
            &[],
        );
        assert!(
            !cache.preview_audition_warm_needed(&path),
            "a completed warm attempt should stay ineligible even if no clip was produced"
        );
    }

    #[test]
    fn preview_warm_cancel_does_not_requeue_scheduled_paths() {
        let path = PathBuf::from("/tmp/wavecrate-preview-cancelled.wav");
        let path_id = path.display().to_string();
        let mut cache = WaveformCacheState::default();

        cache.mark_preview_audition_warm_scheduled(std::slice::from_ref(&path_id));
        assert!(!cache.preview_audition_warm_needed(&path));
        cache.cancel_preview_audition_warm_schedule();

        assert!(
            !cache.preview_audition_warm_needed(&path),
            "cancelled background warm work must not churn on the same visible path"
        );
    }

    #[test]
    fn preview_warm_partial_finish_does_not_requeue_unattempted_tail() {
        let attempted = PathBuf::from("/tmp/wavecrate-preview-attempted.wav");
        let skipped = PathBuf::from("/tmp/wavecrate-preview-skipped.wav");
        let attempted_id = attempted.display().to_string();
        let skipped_id = skipped.display().to_string();
        let mut cache = WaveformCacheState::default();

        cache.mark_preview_audition_warm_scheduled(&[attempted_id.clone(), skipped_id.clone()]);
        cache.finish_preview_audition_warm_schedule(
            &[attempted_id, skipped_id],
            &[attempted.display().to_string()],
            &[],
        );

        assert!(!cache.preview_audition_warm_needed(&attempted));
        assert!(
            !cache.preview_audition_warm_needed(&skipped),
            "scheduled-but-unattempted warm tails should not be rediscovered every frame"
        );
        assert!(
            cache.preview_audition_decode_needed(&skipped),
            "foreground drag playback can still decode a path skipped by background warming"
        );
    }

    #[test]
    fn confirmed_preview_failure_is_not_retried_by_foreground_decode() {
        let path = PathBuf::from("/tmp/wavecrate-preview-failed.wav");
        let mut cache = WaveformCacheState::default();

        assert!(cache.preview_audition_decode_needed(&path));
        cache.mark_preview_audition_failed(&path);

        assert!(
            !cache.preview_audition_decode_needed(&path),
            "interactive preview decode should not churn on a path that already failed preview decoding"
        );
        assert!(
            !cache.preview_audition_warm_needed(&path),
            "confirmed preview failures should remain out of background warm planning"
        );
    }

    #[test]
    fn warm_failed_path_is_not_retried_by_foreground_decode() {
        let path = PathBuf::from("/tmp/wavecrate-preview-warm-failed.wav");
        let path_id = path.display().to_string();
        let mut cache = WaveformCacheState::default();

        cache.mark_preview_audition_warm_scheduled(std::slice::from_ref(&path_id));
        cache.finish_preview_audition_warm_schedule(
            std::slice::from_ref(&path_id),
            std::slice::from_ref(&path_id),
            std::slice::from_ref(&path_id),
        );

        assert!(
            !cache.preview_audition_decode_needed(&path),
            "foreground drag/list/keyboard playback should skip known failed preview heads"
        );
    }

    #[test]
    fn preview_head_cache_does_not_mark_sample_fully_instant_ready() {
        let path = PathBuf::from("/tmp/wavecrate-preview-head.wav");
        let path_id = path.display().to_string();
        let mut cache = WaveformCacheState::default();

        cache.store_preview_audition_clip(preview_clip(path.clone()));

        assert!(cache.preview_audition_clips.contains_key(&path));
        assert!(
            !cache.instant_audition_sample_paths.contains(&path_id),
            "a tiny preview head should not make the UI advertise full instant-audition readiness"
        );
    }

    #[test]
    fn preview_cache_eviction_does_not_immediately_requeue_warm() {
        let path = PathBuf::from("/tmp/wavecrate-preview-evicted.wav");
        let mut cache = WaveformCacheState::default();

        cache.store_preview_audition_clip(preview_clip(path.clone()));
        cache.evict_preview_audition_clip(&path);

        assert!(
            !cache.preview_audition_warm_needed(&path),
            "background preview warm should not churn on evicted cache entries"
        );
    }
}
