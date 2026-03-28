use super::super::*;

const MIN_SCROLL_SPEED: f32 = 0.2;
const MAX_SCROLL_SPEED: f32 = 5.0;
const MIN_ZOOM_FACTOR: f32 = 0.5;
const MAX_ZOOM_FACTOR: f32 = 0.995;
const WHEEL_ZOOM_ANCHOR_FACTOR: f32 = 0.96;
const MIN_WHEEL_ZOOM_SPEED: f32 = 0.1;
const MAX_WHEEL_ZOOM_SPEED: f32 = 20.0;
const MIN_ANTI_CLIP_FADE_MS: f32 = 0.0;
const MAX_ANTI_CLIP_FADE_MS: f32 = 20.0;

pub(crate) fn clamp_scroll_speed(speed: f32) -> f32 {
    speed.clamp(MIN_SCROLL_SPEED, MAX_SCROLL_SPEED)
}

pub(crate) fn clamp_zoom_factor(factor: f32) -> f32 {
    factor.clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR)
}

pub(crate) fn clamp_anti_clip_fade_ms(fade_ms: f32) -> f32 {
    fade_ms.clamp(MIN_ANTI_CLIP_FADE_MS, MAX_ANTI_CLIP_FADE_MS)
}

fn clamp_wheel_zoom_speed(speed: f32) -> f32 {
    speed.clamp(MIN_WHEEL_ZOOM_SPEED, MAX_WHEEL_ZOOM_SPEED)
}

pub(super) fn wheel_zoom_speed_to_factor(speed: f32) -> f32 {
    let speed = clamp_wheel_zoom_speed(speed);
    clamp_zoom_factor(WHEEL_ZOOM_ANCHOR_FACTOR.powf(speed))
}

pub(super) fn wheel_zoom_factor_to_speed(factor: f32) -> f32 {
    let factor = clamp_zoom_factor(factor);
    clamp_wheel_zoom_speed(factor.ln() / WHEEL_ZOOM_ANCHOR_FACTOR.ln())
}

impl AppController {
    /// Set and persist waveform scroll speed (clamped).
    pub fn set_waveform_scroll_speed(&mut self, speed: f32) {
        let clamped = clamp_scroll_speed(speed);
        if (self.settings.controls.waveform_scroll_speed - clamped).abs() < f32::EPSILON {
            return;
        }
        self.settings.controls.waveform_scroll_speed = clamped;
        self.ui.controls.waveform_scroll_speed = clamped;
        self.persist_controls();
    }

    /// Toggle and persist inverted waveform scroll direction.
    pub fn set_invert_waveform_scroll(&mut self, invert: bool) {
        if self.settings.controls.invert_waveform_scroll == invert {
            return;
        }
        self.settings.controls.invert_waveform_scroll = invert;
        self.ui.controls.invert_waveform_scroll = invert;
        self.persist_controls();
    }

    /// Set and persist wheel zoom factor (clamped).
    pub fn set_wheel_zoom_factor(&mut self, factor: f32) {
        let clamped = clamp_zoom_factor(factor);
        if (self.settings.controls.wheel_zoom_factor - clamped).abs() < f32::EPSILON {
            return;
        }
        self.settings.controls.wheel_zoom_factor = clamped;
        self.ui.controls.wheel_zoom_factor = clamped;
        self.persist_controls();
    }

    /// Return the derived wheel zoom speed from the stored factor.
    pub fn wheel_zoom_speed(&self) -> f32 {
        wheel_zoom_factor_to_speed(self.ui.controls.wheel_zoom_factor)
    }

    /// Set and persist wheel zoom speed (low = slower, high = faster).
    pub fn set_wheel_zoom_speed(&mut self, speed: f32) {
        self.set_wheel_zoom_factor(wheel_zoom_speed_to_factor(speed));
    }

    /// Set and persist keyboard zoom factor (clamped).
    pub fn set_keyboard_zoom_factor(&mut self, factor: f32) {
        let clamped = clamp_zoom_factor(factor);
        if (self.settings.controls.keyboard_zoom_factor - clamped).abs() < f32::EPSILON {
            return;
        }
        self.settings.controls.keyboard_zoom_factor = clamped;
        self.ui.controls.keyboard_zoom_factor = clamped;
        self.persist_controls();
    }

    /// Toggle and persist auto edge fades for new samples exported from selections.
    pub fn set_auto_edge_fades_on_selection_exports(&mut self, enabled: bool) {
        if self.settings.controls.auto_edge_fades_on_selection_exports == enabled {
            return;
        }
        self.settings.controls.auto_edge_fades_on_selection_exports = enabled;
        self.ui.controls.auto_edge_fades_on_selection_exports = enabled;
        self.persist_controls();
    }

    /// Toggle and persist destructive "yolo mode" (skip confirmation prompts).
    pub fn set_destructive_yolo_mode(&mut self, enabled: bool) {
        if self.settings.controls.destructive_yolo_mode == enabled {
            return;
        }
        self.settings.controls.destructive_yolo_mode = enabled;
        self.ui.controls.destructive_yolo_mode = enabled;
        self.persist_controls();
    }

    /// Enable/disable BPM snapping and persist the setting.
    pub fn set_bpm_snap_enabled(&mut self, enabled: bool) {
        let settings_match = self.settings.controls.bpm_snap_enabled == enabled;
        let ui_match = self.ui.waveform.bpm_snap_enabled == enabled;
        if settings_match && ui_match {
            return;
        }
        self.settings.controls.bpm_snap_enabled = enabled;
        self.ui.waveform.bpm_snap_enabled = enabled;
        if !settings_match {
            self.persist_controls();
        }
    }

    /// Enable/disable selection-relative BPM grid anchoring and persist the setting.
    pub fn set_relative_bpm_grid_enabled(&mut self, enabled: bool) {
        let settings_match = self.settings.controls.relative_bpm_grid_enabled == enabled;
        let ui_match = self.ui.waveform.relative_bpm_grid_enabled == enabled;
        if settings_match && ui_match {
            return;
        }
        self.settings.controls.relative_bpm_grid_enabled = enabled;
        self.ui.waveform.relative_bpm_grid_enabled = enabled;
        if !settings_match {
            self.persist_controls();
        }
    }

    /// Enable/disable BPM auto-override lock and persist the setting.
    pub fn set_bpm_lock_enabled(&mut self, enabled: bool) {
        if self.settings.controls.bpm_lock_enabled == enabled {
            return;
        }
        self.settings.controls.bpm_lock_enabled = enabled;
        self.ui.waveform.bpm_lock_enabled = enabled;
        self.persist_controls();
    }

    /// Enable/disable loop lock and persist the setting.
    pub fn set_loop_lock_enabled(&mut self, enabled: bool) {
        if self.settings.controls.loop_lock_enabled == enabled {
            return;
        }
        self.settings.controls.loop_lock_enabled = enabled;
        self.ui.waveform.loop_lock_enabled = enabled;
        self.persist_controls();
    }

    /// Enable/disable transient snapping and persist the setting.
    pub fn set_transient_snap_enabled(&mut self, enabled: bool) {
        if self.settings.controls.transient_snap_enabled == enabled {
            self.ui.waveform.transient_snap_enabled =
                enabled && self.ui.waveform.transient_markers_enabled;
            return;
        }
        self.settings.controls.transient_snap_enabled = enabled;
        self.ui.waveform.transient_snap_enabled =
            enabled && self.ui.waveform.transient_markers_enabled;
        self.persist_controls();
    }

    /// Enable/disable transient marker rendering and persist the setting.
    pub fn set_transient_markers_enabled(&mut self, enabled: bool) {
        let expected_transient_snap_enabled = if enabled {
            self.settings.controls.transient_snap_enabled
        } else {
            false
        };
        let settings_match = self.settings.controls.transient_markers_enabled == enabled;
        let ui_match = self.ui.waveform.transient_markers_enabled == enabled
            && self.ui.waveform.transient_snap_enabled == expected_transient_snap_enabled;
        if settings_match && ui_match {
            return;
        }
        self.settings.controls.transient_markers_enabled = enabled;
        self.ui.waveform.transient_markers_enabled = enabled;
        self.ui.waveform.transient_snap_enabled = expected_transient_snap_enabled;
        if !settings_match {
            self.persist_controls();
        }
    }

    /// Enable/disable normalized audition playback and persist the setting.
    pub fn set_normalized_audition_enabled(&mut self, enabled: bool) {
        let settings_match = self.settings.controls.normalized_audition_enabled == enabled;
        let ui_match = self.ui.waveform.normalized_audition_enabled == enabled;
        if settings_match && ui_match {
            return;
        }
        self.settings.controls.normalized_audition_enabled = enabled;
        self.ui.waveform.normalized_audition_enabled = enabled;
        if !settings_match {
            self.persist_controls();
        }
    }

    /// Enable/disable waveform slice mode for pointer-based slice editing.
    ///
    /// Slice mode is UI-only state, so it is not persisted in the user config.
    pub fn set_slice_mode_enabled(&mut self, enabled: bool) {
        if self.ui.waveform.slice_mode_enabled == enabled {
            return;
        }
        self.ui.waveform.slice_mode_enabled = enabled;
        if !enabled {
            self.ui.waveform.selected_slices.clear();
            self.ui.waveform.slice_review = Default::default();
        }
    }

    /// Set and persist the tooltip detail level.
    pub fn set_tooltip_mode(&mut self, mode: crate::sample_sources::config::TooltipMode) {
        if self.settings.controls.tooltip_mode == mode {
            return;
        }
        self.settings.controls.tooltip_mode = mode;
        self.ui.controls.tooltip_mode = mode;
        self.persist_controls();
    }

    /// Toggle and persist auto-advance after rating/tagging.
    pub fn set_advance_after_rating(&mut self, enabled: bool) {
        if self.settings.controls.advance_after_rating == enabled {
            return;
        }
        self.settings.controls.advance_after_rating = enabled;
        self.ui.controls.advance_after_rating = enabled;
        self.persist_controls();
    }

    pub(super) fn persist_controls(&mut self) {
        if let Err(err) = self.persist_config("Failed to save options") {
            self.set_status(err, StatusTone::Warning);
        }
    }
}
