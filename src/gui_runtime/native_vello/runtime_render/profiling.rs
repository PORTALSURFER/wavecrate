use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    fn maybe_record_redraw_profile(
        &mut self,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        total: Duration,
    ) {
        let text_profile = if self.profiler.is_enabled() {
            self.text_renderer.take_layout_profile_counters()
        } else {
            (0, 0, 0, 0, 0, 0)
        };
        self.profiler
            .record_redraw(rebuild, acquire, render, blit, present, total, text_profile);
    }

    /// Build per-frame renderer counts shared with bridge-side telemetry.
    pub(in crate::gui_runtime::native_vello) fn frame_result_base(&self) -> FrameBuildResult {
        FrameBuildResult {
            primitive_count: self
                .frame_cache
                .primitives
                .len()
                .saturating_add(self.hover_overlay_frame_cache.primitives.len())
                .saturating_add(self.focus_overlay_frame_cache.primitives.len())
                .saturating_add(self.modal_overlay_frame_cache.primitives.len())
                .saturating_add(self.waveform_motion_overlay_frame_cache.primitives.len())
                .saturating_add(self.chrome_motion_overlay_frame_cache.primitives.len()),
            text_run_count: self
                .frame_cache
                .text_runs
                .len()
                .saturating_add(self.hover_overlay_frame_cache.text_runs.len())
                .saturating_add(self.focus_overlay_frame_cache.text_runs.len())
                .saturating_add(self.modal_overlay_frame_cache.text_runs.len())
                .saturating_add(self.waveform_motion_overlay_frame_cache.text_runs.len())
                .saturating_add(self.chrome_motion_overlay_frame_cache.text_runs.len()),
            needs_animation: self.shell_state.needs_animation(),
            ..FrameBuildResult::default()
        }
    }

    /// Build one frame-result payload with redraw attribution bits set.
    pub(in crate::gui_runtime::native_vello) fn frame_result_with_rebuilds(
        &self,
        layout_rebuild: bool,
        static_rebuild: bool,
        state_overlay_rebuild: bool,
        motion_overlay_rebuild: bool,
    ) -> FrameBuildResult {
        let mut result = self.frame_result_base();
        result.layout_rebuild = layout_rebuild;
        result.static_rebuild = static_rebuild;
        result.state_overlay_rebuild = state_overlay_rebuild;
        result.motion_overlay_rebuild = motion_overlay_rebuild;
        result
    }

    /// Convert one duration to microseconds while saturating at `u32::MAX`.
    fn duration_us_u32(duration: Duration) -> u32 {
        duration.as_micros().min(u128::from(u32::MAX)) as u32
    }

    /// Return the configured redraw frame budget in microseconds.
    fn frame_budget_us(&self) -> u32 {
        Self::duration_us_u32(self.target_frame_interval)
    }

    /// Finalize and emit one frame result payload to the host bridge.
    fn emit_frame_result(
        &mut self,
        frame_result: &mut FrameBuildResult,
        frame_total: Duration,
        present: Duration,
        presented: bool,
        present_expected: bool,
    ) {
        let frame_budget_us = self.frame_budget_us();
        let frame_total_us = Self::duration_us_u32(frame_total);
        frame_result.frame_total_us = frame_total_us;
        frame_result.present_us = Self::duration_us_u32(present);
        frame_result.frame_budget_us = frame_budget_us;
        frame_result.presented = presented;
        frame_result.missed_present = present_expected && !presented;
        frame_result.jank = presented && frame_total_us > frame_budget_us;
        self.bridge.observe_frame_result(*frame_result);
    }

    /// Record profiler data (if enabled) and emit one finalized frame result.
    pub(in crate::gui_runtime::native_vello) fn finish_redraw_attempt(
        &mut self,
        frame_result: &mut FrameBuildResult,
        frame_started_at: Instant,
        frame_profile_start: Option<Instant>,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        presented: bool,
        present_expected: bool,
    ) {
        if let Some(start) = frame_profile_start {
            self.maybe_record_redraw_profile(
                rebuild,
                acquire,
                render,
                blit,
                present,
                start.elapsed(),
            );
        }
        self.emit_frame_result(
            frame_result,
            frame_started_at.elapsed(),
            present,
            presented,
            present_expected,
        );
    }
}
