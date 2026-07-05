use radiant::prelude as ui;
use std::time::{Duration, Instant};

use crate::native_app::app::{GuiMessage, NativeAppState, sample_path_label};

const SLOW_FRAME_DISPATCH_PROFILE_THRESHOLD: Duration = Duration::from_micros(16_667);

#[derive(Default)]
struct FrameDispatchProfile {
    app_icon_ms: Duration,
    audio_player_ms: Duration,
    startup_source_scan_ms: Duration,
    pending_source_refresh_ms: Duration,
    startup_auto_load_ms: Duration,
    release_update_check_ms: Duration,
    waveform_cache_warm_ms: Duration,
    active_folder_cache_warm_ms: Duration,
    starmap_layout_load_ms: Duration,
    preview_audition_warm_ms: Duration,
    playback_retarget_ms: Duration,
    advance_frame_ms: Duration,
}

impl NativeAppState {
    pub(super) fn apply_frame_message(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
        let total_started_at = Instant::now();
        let mut profile = FrameDispatchProfile::default();
        profile.app_icon_ms = measure_frame_phase(|| self.maybe_install_application_icon());
        profile.audio_player_ms = measure_frame_phase(|| self.maybe_open_audio_player(context));
        profile.startup_source_scan_ms =
            measure_frame_phase(|| self.maybe_startup_source_scan(context));
        profile.pending_source_refresh_ms =
            measure_frame_phase(|| self.maybe_run_pending_source_refresh(context));
        profile.startup_auto_load_ms =
            measure_frame_phase(|| self.maybe_auto_load_startup_sample(context));
        profile.release_update_check_ms =
            measure_frame_phase(|| self.maybe_start_release_update_check(context));
        profile.waveform_cache_warm_ms =
            measure_frame_phase(|| self.maybe_start_waveform_cache_warm(context));
        profile.active_folder_cache_warm_ms =
            measure_frame_phase(|| self.maybe_start_active_folder_cache_warm(context));
        profile.starmap_layout_load_ms =
            measure_frame_phase(|| self.maybe_start_starmap_layout_load(context));
        profile.preview_audition_warm_ms =
            measure_frame_phase(|| self.maybe_start_preview_audition_warm(context));
        profile.playback_retarget_ms =
            measure_frame_phase(|| self.flush_pending_play_selection_playback_retarget());
        profile.advance_frame_ms = measure_frame_phase(|| self.advance_frame(context));
        self.log_slow_frame_dispatch_profile(total_started_at.elapsed(), &profile);
    }

    fn maybe_install_application_icon(&mut self) {
        if !self.ui.startup.app_icon_install_pending {
            return;
        }
        self.ui.startup.app_icon_install_pending = false;
        crate::native_app::shell::macos_app_icon::install_wavecrate_application_icon();
    }

    fn log_slow_frame_dispatch_profile(&self, total: Duration, profile: &FrameDispatchProfile) {
        if total < SLOW_FRAME_DISPATCH_PROFILE_THRESHOLD {
            return;
        }
        let selected = self
            .library
            .folder_browser
            .selected_file_id()
            .map(sample_path_label)
            .unwrap_or_default();
        tracing::warn!(
            target: "wavecrate::debug::ui_frame",
            event = "ui.frame.dispatch_profile",
            elapsed_ms = super::duration_ms(total),
            app_icon_ms = super::duration_ms(profile.app_icon_ms),
            audio_player_ms = super::duration_ms(profile.audio_player_ms),
            startup_source_scan_ms = super::duration_ms(profile.startup_source_scan_ms),
            pending_source_refresh_ms = super::duration_ms(profile.pending_source_refresh_ms),
            startup_auto_load_ms = super::duration_ms(profile.startup_auto_load_ms),
            release_update_check_ms = super::duration_ms(profile.release_update_check_ms),
            waveform_cache_warm_ms = super::duration_ms(profile.waveform_cache_warm_ms),
            active_folder_cache_warm_ms = super::duration_ms(profile.active_folder_cache_warm_ms),
            starmap_layout_load_ms = super::duration_ms(profile.starmap_layout_load_ms),
            preview_audition_warm_ms = super::duration_ms(profile.preview_audition_warm_ms),
            playback_retarget_ms = super::duration_ms(profile.playback_retarget_ms),
            advance_frame_ms = super::duration_ms(profile.advance_frame_ms),
            sample_loading = self.active_sample_load_task().is_some(),
            waveform_loading = self.waveform_sample_load_active(),
            playing = self.playback_visual_activity_active(),
            pending_playback = self.audio.pending_playback_start.is_some(),
            selected = selected.as_str(),
            "Slow UI frame dispatch profile"
        );
    }
}

fn measure_frame_phase(phase: impl FnOnce()) -> Duration {
    let started_at = Instant::now();
    phase();
    started_at.elapsed()
}
