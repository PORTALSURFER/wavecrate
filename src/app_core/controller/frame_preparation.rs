use crate::app_core::app_api::controller_state::DerivedNodeId;

use super::AppController;

/// Internal frame-preparation plans used by the UI bridge.
///
/// The controller still exposes `prepare_native_frame(bool)` as the stable runtime
/// API, but bridge pulls can choose a narrower maintenance lane when the pending
/// state shows that only browser-local work needs to run before projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeFramePreparationPlan {
    /// Run the full maintenance pass before projecting a model pull.
    Full,
    /// Run only the browser/status-safe subset for retained browser pulls.
    BrowserRetainedPull,
    /// Run browser plus deferred transport/status maintenance without the full pass.
    TransportRetainedPull,
    /// Run browser plus deferred metadata maintenance without the full pass.
    MetadataRetainedPull,
    /// Run browser plus deferred startup maintenance without the full pass.
    StartupRetainedPull,
    /// Run the animation-only maintenance pass for motion-model pulls.
    MotionOnly,
}

impl AppController {
    /// Execute one internal native-frame preparation plan.
    pub(crate) fn prepare_native_frame_with_plan(&mut self, plan: NativeFramePreparationPlan) {
        self.poll_background_jobs();
        match plan {
            NativeFramePreparationPlan::Full => {
                self.flush_transport_native_frame_lane();
                self.flush_browser_native_frame_lane();
                self.flush_metadata_native_frame_lane();
                self.flush_waveform_native_frame_lane();
                self.flush_startup_native_frame_lane();
                self.tick_playhead();
                self.finish_non_motion_native_frame_preparation();
            }
            NativeFramePreparationPlan::BrowserRetainedPull => {
                self.flush_browser_native_frame_lane();
                self.finish_non_motion_native_frame_preparation();
            }
            NativeFramePreparationPlan::TransportRetainedPull => {
                self.flush_transport_native_frame_lane();
                self.flush_browser_native_frame_lane();
                self.finish_non_motion_native_frame_preparation();
            }
            NativeFramePreparationPlan::MetadataRetainedPull => {
                self.flush_browser_native_frame_lane();
                self.flush_metadata_native_frame_lane();
                self.finish_non_motion_native_frame_preparation();
            }
            NativeFramePreparationPlan::StartupRetainedPull => {
                self.flush_browser_native_frame_lane();
                self.flush_startup_native_frame_lane();
                self.finish_non_motion_native_frame_preparation();
            }
            NativeFramePreparationPlan::MotionOnly => {
                self.record_frame_timing_for_fps();
                if !self.is_playing() {
                    let _ = self.refresh_projection_revision_bus();
                    return;
                }
                self.tick_playhead();
                let _ = self.refresh_projection_revision_bus();
            }
        }
    }

    /// Return whether the bridge may use the browser-retained maintenance lane.
    ///
    /// This path is intentionally conservative: any queued transport, waveform,
    /// metadata, startup, map, or playback-sensitive work keeps the next pull on
    /// the full preparation lane.
    pub(crate) fn can_prepare_browser_retained_pull(&self) -> bool {
        self.can_prepare_retained_pull_base()
            && !self.has_transport_native_frame_work()
            && !self.has_metadata_native_frame_work()
            && !self.has_startup_native_frame_work()
    }

    /// Return whether the bridge may use the transport-retained maintenance lane.
    pub(crate) fn can_prepare_transport_retained_pull(&self) -> bool {
        self.can_prepare_retained_pull_base()
            && self.has_transport_native_frame_work()
            && !self.has_metadata_native_frame_work()
            && !self.has_startup_native_frame_work()
    }

    /// Return whether the bridge may use the metadata-retained maintenance lane.
    pub(crate) fn can_prepare_metadata_retained_pull(&self) -> bool {
        self.can_prepare_retained_pull_base()
            && self.has_metadata_native_frame_work()
            && !self.has_transport_native_frame_work()
            && !self.has_startup_native_frame_work()
    }

    /// Return whether the bridge may use the startup-retained maintenance lane.
    pub(crate) fn can_prepare_startup_retained_pull(&self) -> bool {
        self.can_prepare_retained_pull_base()
            && self.has_startup_native_frame_work()
            && !self.has_transport_native_frame_work()
            && !self.has_metadata_native_frame_work()
    }

    /// Flush native-frame transport maintenance that can affect persisted runtime state.
    fn flush_transport_native_frame_lane(&mut self) {
        if self.has_pending_volume_setting_flush() {
            self.flush_pending_volume_setting();
        }
    }

    /// Flush native-frame browser/status maintenance needed by retained browser pulls.
    fn flush_browser_native_frame_lane(&mut self) {
        if self.has_pending_age_update_commit() {
            self.flush_pending_age_update_commit();
        }
        if self.has_pending_focused_similarity_highlight_refresh() {
            self.flush_pending_focused_similarity_highlight_refresh();
        }
        if self.has_pending_browser_focus_commit() {
            self.flush_pending_browser_focus_commit();
        }
    }

    /// Flush deferred metadata writes owned by the controller.
    fn flush_metadata_native_frame_lane(&mut self) {
        if self.has_pending_loaded_duration_metadata_write() {
            self.flush_pending_loaded_duration_metadata_write();
        }
    }

    /// Flush waveform work that can change rendered pixels or playback targets.
    fn flush_waveform_native_frame_lane(&mut self) {
        if self.has_pending_waveform_seek_commit() {
            self.flush_pending_waveform_seek_commit();
        }
        if self.has_pending_waveform_image_refresh() {
            self.flush_pending_waveform_image_refresh();
        }
    }

    /// Flush deferred startup work once the runtime is ready to expose it.
    fn flush_startup_native_frame_lane(&mut self) {
        if self.has_pending_startup_source_db_maintenance() {
            self.flush_deferred_startup_source_db_maintenance();
        }
        if self.has_pending_startup_audio_refresh() {
            self.flush_deferred_startup_audio_refresh();
        }
    }

    /// Finish a non-motion native frame preparation pass.
    fn finish_non_motion_native_frame_preparation(&mut self) {
        let _ = self.refresh_projection_revision_bus();
        self.update_performance_governor(false);
    }

    /// Return true when a retained pull may skip full playhead, waveform, and map work.
    fn can_prepare_retained_pull_base(&self) -> bool {
        !self.is_playing()
            && !self.has_waveform_native_frame_work()
            && !self.is_derived_node_dirty(DerivedNodeId::MapState)
    }

    /// Return true when queued transport work still needs a frame-time flush.
    fn has_transport_native_frame_work(&self) -> bool {
        self.has_pending_volume_setting_flush()
            || self.is_derived_node_dirty(DerivedNodeId::TransportState)
    }

    /// Return true when queued metadata work still needs a frame-time flush.
    fn has_metadata_native_frame_work(&self) -> bool {
        self.has_pending_loaded_duration_metadata_write()
    }

    /// Return true when queued waveform work still needs a frame-time flush.
    fn has_waveform_native_frame_work(&self) -> bool {
        self.has_pending_waveform_seek_commit()
            || self.has_pending_waveform_image_refresh()
            || self.is_derived_node_dirty(DerivedNodeId::WaveformState)
    }

    /// Return true when queued startup work still needs a frame-time flush.
    fn has_startup_native_frame_work(&self) -> bool {
        self.has_pending_startup_source_db_maintenance() || self.has_pending_startup_audio_refresh()
    }
}
