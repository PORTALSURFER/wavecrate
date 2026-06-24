use super::*;
use crate::audio::short_edge_fade_frame_count;

impl AppController {
    /// Remove the selected span from the loaded sample.
    pub(crate) fn trim_waveform_selection(&mut self) -> Result<(), String> {
        self.apply_or_queue_selection_edit(
            "Trimmed selection",
            false,
            SelectionEditWorkerOp::Trim,
            trim_buffer,
        )
    }

    /// Fade the selected span down to silence using the given direction.
    pub(crate) fn fade_waveform_selection(
        &mut self,
        direction: FadeDirection,
    ) -> Result<(), String> {
        if !cfg!(test) {
            return self.queue_selection_edit_commit(
                "Applied fade",
                format!(
                    "Applied fade {}",
                    self.selection_target()?.relative_path.display()
                ),
                true,
                false,
                false,
                SelectionEditWorkerOp::Fade { direction },
            );
        }
        self.apply_selection_edit_with_status("Applied fade", true, |buffer| {
            apply_directional_fade(
                &mut buffer.samples,
                buffer.channels,
                buffer.start_frame,
                buffer.end_frame,
                direction,
            );
            Ok(())
        })
    }

    /// Normalize the active selection and apply short fades at the edges.
    pub(crate) fn normalize_waveform_selection(&mut self) -> Result<(), String> {
        self.apply_or_queue_selection_edit(
            "Normalized selection",
            true,
            SelectionEditWorkerOp::Normalize {
                edge_fade: Duration::from_millis(5),
            },
            |buffer| normalize_selection(buffer, Duration::from_millis(5)),
        )
    }

    /// Apply short fade-in/out ramps at the selection edges to reduce clicks.
    pub(crate) fn soften_waveform_selection_edges(&mut self) -> Result<(), String> {
        let fade_duration =
            Duration::from_secs_f32(self.ui.controls.anti_clip_fade_ms.max(0.0) / 1000.0);
        if !cfg!(test) {
            return self.queue_selection_edit_commit(
                "Applied short fades",
                format!(
                    "Applied short fades {}",
                    self.selection_target()?.relative_path.display()
                ),
                true,
                false,
                false,
                SelectionEditWorkerOp::ShortEdgeFades { fade_duration },
            );
        }
        self.apply_selection_edit_with_status("Applied short fades", true, |buffer| {
            apply_short_edge_fades_to_selection(buffer, fade_duration)
        })
    }

    /// Repair clicks inside the selection by interpolating the span.
    pub(crate) fn repair_clicks_selection(&mut self) -> Result<(), String> {
        self.apply_or_queue_selection_edit(
            "Removed clicks",
            true,
            SelectionEditWorkerOp::RepairClicks,
            repair_clicks_buffer,
        )
    }

    /// Silence the selected span without applying fades.
    pub(crate) fn mute_waveform_selection(&mut self) -> Result<(), String> {
        self.apply_or_queue_selection_edit(
            "Muted selection",
            true,
            SelectionEditWorkerOp::Mute,
            ops::mute_buffer,
        )
    }

    /// Reverse the selected span in time.
    pub(crate) fn reverse_waveform_selection(&mut self) -> Result<(), String> {
        self.apply_or_queue_selection_edit(
            "Reversed selection",
            true,
            SelectionEditWorkerOp::Reverse,
            reverse_buffer,
        )
    }

    fn apply_or_queue_selection_edit(
        &mut self,
        status: &'static str,
        refresh_preview: bool,
        worker_op: SelectionEditWorkerOp,
        apply_now: impl FnMut(&mut SelectionEditBuffer) -> Result<(), String>,
    ) -> Result<(), String> {
        if !cfg!(test) {
            return self.queue_selection_edit_commit(
                status,
                format!(
                    "{} {}",
                    status,
                    self.selection_target()?.relative_path.display()
                ),
                refresh_preview,
                false,
                false,
                worker_op,
            );
        }
        self.apply_selection_edit_with_status(status, refresh_preview, apply_now)
    }

    fn apply_selection_edit_with_status(
        &mut self,
        status: &'static str,
        refresh_preview: bool,
        apply_now: impl FnMut(&mut SelectionEditBuffer) -> Result<(), String>,
    ) -> Result<(), String> {
        let result = self.apply_selection_edit(status, refresh_preview, apply_now);
        if let Err(err) = &result {
            self.set_status(err.clone(), StatusTone::Error);
        }
        result
    }
}

fn apply_short_edge_fades_to_selection(
    buffer: &mut SelectionEditBuffer,
    fade_duration: Duration,
) -> Result<(), String> {
    let selection_frames = buffer.end_frame.saturating_sub(buffer.start_frame);
    let fade_frames =
        short_edge_fade_frame_count(buffer.sample_rate.max(1), selection_frames, fade_duration);
    if fade_frames == 0 {
        return Err("Selection is too short for edge fades".into());
    }
    apply_edge_fades(
        &mut buffer.samples,
        buffer.channels,
        buffer.start_frame,
        buffer.end_frame,
        fade_frames,
    );
    Ok(())
}
