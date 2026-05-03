//! Model-sync, editor-state, and animation bookkeeping for the native shell.

use super::*;
use crate::compat_app_contract::{FolderPaneIdModel, StatusChipStateModel};

impl NativeShellState {
    /// Synchronize local interaction state from the latest app model.
    pub(crate) fn sync_from_model(&mut self, model: &AppModel) {
        self.selected_column = model.selected_column.min(2);
        self.transport_running = model.transport_running;
        self.status_options_button_error =
            model.paired_device_panel().status_state() == StatusChipStateModel::Error;
        self.startup_frame_ticks = self.startup_frame_ticks.saturating_sub(1);
        if model.map.active {
            self.hovered_browser_visible_row = None;
        }
        if self
            .hovered_folder_row_index
            .zip(self.hovered_folder_pane)
            .is_some_and(|(row_index, pane)| {
                row_index >= model.sources.folder_pane(pane).tree_rows.len()
            })
        {
            self.hovered_folder_pane = None;
            self.hovered_folder_row_index = None;
        }
        sync_folder_pane_model(
            &mut self.upper_folder_pane,
            model.sources.folder_pane(FolderPaneIdModel::Upper),
        );
        sync_folder_pane_model(
            &mut self.lower_folder_pane,
            model.sources.folder_pane(FolderPaneIdModel::Lower),
        );
        if self
            .source_context_menu
            .is_some_and(|menu| menu.row_index >= model.sources.rows.len())
        {
            self.source_context_menu = None;
        }
        if self
            .browser_context_menu
            .is_some_and(|menu| menu.visible_row >= model.browser.rows.len())
        {
            self.browser_context_menu = None;
        }
        self.has_focus_emphasis = model.focus_context
            != crate::compat_app_contract::FocusContextModel::None
            || model
                .browser
                .rows
                .iter()
                .any(|row| row.focused || row.selected)
            || model.sources.rows.iter().any(|row| row.selected)
            || model
                .sources
                .upper_folder_pane
                .tree_rows
                .iter()
                .chain(model.sources.lower_folder_pane.tree_rows.iter())
                .any(|row| row.focused || row.selected);
    }

    /// Synchronize motion-sensitive state from a dedicated motion model projection.
    pub(crate) fn sync_from_motion_model(&mut self, model: &NativeMotionModel) {
        self.transport_running = model.transport_running;
        if model.waveform_selection_export_flash_nonce
            != self.last_waveform_selection_export_flash_nonce
        {
            self.last_waveform_selection_export_flash_nonce =
                model.waveform_selection_export_flash_nonce;
            self.trigger_waveform_selection_flash(WaveformSelectionFlashTone::Optimistic);
        }
        if model.waveform_selection_export_failure_flash_nonce
            != self.last_waveform_selection_export_failure_flash_nonce
        {
            self.last_waveform_selection_export_failure_flash_nonce =
                model.waveform_selection_export_failure_flash_nonce;
            self.trigger_waveform_selection_flash(WaveformSelectionFlashTone::Error);
        }
        if model.waveform_edit_selection_apply_flash_nonce
            != self.last_waveform_edit_selection_apply_flash_nonce
        {
            self.last_waveform_edit_selection_apply_flash_nonce =
                model.waveform_edit_selection_apply_flash_nonce;
            self.trigger_waveform_edit_selection_flash();
        }
    }

    /// Update waveform BPM toolbar editor state used by toolbar rendering.
    pub(crate) fn set_waveform_bpm_editor_state(
        &mut self,
        active: bool,
        display_text: Option<String>,
        visual: Option<TextFieldVisualState>,
    ) {
        if self.waveform_bpm_input_active == active
            && self.waveform_bpm_input_display == display_text
            && self.waveform_bpm_editor_visual == visual
        {
            return;
        }
        self.waveform_bpm_input_active = active;
        self.waveform_bpm_input_display = display_text;
        self.waveform_bpm_editor_visual = visual;
        self.waveform_toolbar_hit_test_cache_key = None;
    }

    /// Update the active browser-search editor visuals shown in state overlays.
    pub(crate) fn set_browser_search_editor_state(&mut self, visual: Option<TextFieldVisualState>) {
        self.browser_search_editor_visual = visual;
    }

    /// Update the active browser pill-editor visuals shown in state overlays.
    pub(crate) fn set_browser_pill_editor_visual_state(
        &mut self,
        visual: Option<TextFieldVisualState>,
    ) {
        self.browser_pill_editor_visual = visual;
    }

    /// Update the active inline folder-create editor visuals shown in sidebar overlays.
    pub(crate) fn set_folder_create_editor_state(&mut self, visual: Option<TextFieldVisualState>) {
        self.folder_create_editor_visual = visual;
    }

    /// Clear transient waveform hover feedback during an active drag gesture.
    ///
    /// Resize, shift, and fade drags redraw the waveform overlay on every move.
    /// Leaving the idle hover marker and resize-edge hint active during those
    /// gestures can paint stale guide chrome on top of the live drag target.
    pub(crate) fn clear_waveform_hover_feedback(&mut self) {
        self.hovered_waveform_resize_edge = None;
        self.waveform_hover_x = None;
    }

    pub(super) fn trigger_waveform_toolbar_flash(&mut self, hint: WaveformToolbarHoverHint) {
        self.waveform_toolbar_flash = Some(WaveformToolbarFlash {
            hint,
            ticks_remaining: WAVEFORM_TOOLBAR_FLASH_TICKS,
        });
    }

    pub(super) fn trigger_waveform_selection_flash(&mut self, tone: WaveformSelectionFlashTone) {
        self.waveform_selection_flash_tone = tone;
        self.waveform_selection_flash_ticks = WAVEFORM_SELECTION_FLASH_TICKS;
    }

    pub(super) fn trigger_waveform_edit_selection_flash(&mut self) {
        self.waveform_edit_selection_flash_ticks = WAVEFORM_EDIT_SELECTION_FLASH_TICKS;
    }

    pub(super) fn trigger_source_add_button_flash(&mut self) {
        self.source_add_button_flash_ticks = SOURCE_ADD_BUTTON_FLASH_TICKS;
    }

    pub(super) fn trigger_status_options_button_flash(&mut self) {
        self.status_options_button_flash_ticks = SOURCE_ADD_BUTTON_FLASH_TICKS;
    }

    /// Return the current state-overlay fingerprint.
    #[cfg(test)]
    pub(crate) fn state_overlay_fingerprint(&self) -> StateOverlayFingerprint {
        StateOverlayFingerprint {
            selected_column: self.selected_column,
            hovered: self.hovered,
            hovered_browser_visible_row: self.hovered_browser_visible_row,
            hovered_folder_pane: self.hovered_folder_pane,
            hovered_folder_row_index: self.hovered_folder_row_index,
            hovered_waveform_toolbar_hint: self.hovered_waveform_toolbar_hint,
            browser_search_editor_signature: text_field_visual_signature(
                self.browser_search_editor_visual.as_ref(),
            ),
            browser_search_sidebar_signature: text_field_visual_signature(
                self.browser_pill_editor_visual.as_ref(),
            ),
            folder_create_editor_signature: text_field_visual_signature(
                self.folder_create_editor_visual.as_ref(),
            ),
            has_focus_emphasis: self.has_focus_emphasis,
        }
    }

    /// Return the current motion-overlay fingerprint.
    #[cfg(test)]
    pub(crate) fn motion_overlay_fingerprint(&self) -> MotionOverlayFingerprint {
        MotionOverlayFingerprint {
            transport_running: self.transport_running,
            startup_frame_ticks: self.startup_frame_ticks,
            pulse_phase_bits: self.pulse_phase.to_bits(),
            waveform_hover_x_bits: self.waveform_hover_x.map(f32::to_bits),
            hovered_waveform_resize_edge: self.hovered_waveform_resize_edge,
        }
    }

    /// Return the current waveform-motion overlay fingerprint.
    pub(crate) fn waveform_motion_overlay_fingerprint(&self) -> WaveformMotionOverlayFingerprint {
        WaveformMotionOverlayFingerprint {
            waveform_hover_x_bits: self.waveform_hover_x.map(f32::to_bits),
            hovered_waveform_resize_edge: self.hovered_waveform_resize_edge,
            waveform_selection_flash_active: self.waveform_selection_flash_ticks > 0,
            waveform_edit_selection_flash_active: self.waveform_edit_selection_flash_ticks > 0,
            waveform_selection_flash_tone: self.waveform_selection_flash_tone,
            pulse_phase_bits: self.pulse_phase.to_bits(),
        }
    }

    /// Return the current chrome-motion overlay fingerprint.
    pub(crate) fn chrome_motion_overlay_fingerprint(&self) -> ChromeMotionOverlayFingerprint {
        ChromeMotionOverlayFingerprint {
            transport_running: self.transport_running,
            startup_frame_ticks: self.startup_frame_ticks,
            hovered_browser_rating_filter_level: self.hovered_browser_rating_filter_level,
            hovered_browser_playback_age_filter_chip: self.hovered_browser_playback_age_filter_chip,
            hovered_browser_marked_filter: self.hovered_browser_marked_filter,
            hovered_source_add_button: self.hovered_source_add_button,
            hovered_status_options_button: self.hovered_status_options_button,
            status_options_button_error: self.status_options_button_error,
            hovered_browser_search_field: self.hovered_browser_search_field,
            hovered_waveform_toolbar_hint: self.hovered_waveform_toolbar_hint,
            flashed_source_add_button: self.source_add_button_flash_ticks > 0,
            source_add_button_flash_ticks: self.source_add_button_flash_ticks,
            flashed_status_options_button: self.status_options_button_flash_ticks > 0,
            status_options_button_flash_ticks: self.status_options_button_flash_ticks,
            flashed_waveform_toolbar_hint: self.waveform_toolbar_flash.map(|flash| flash.hint),
            waveform_toolbar_flash_ticks: self
                .waveform_toolbar_flash
                .map_or(0, |flash| flash.ticks_remaining),
            waveform_bpm_editor_signature: text_field_visual_signature(
                self.waveform_bpm_editor_visual.as_ref(),
            ),
            pulse_phase_bits: self.pulse_phase.to_bits(),
        }
    }

    /// Return browser-row truncation lookup counts from the latest row-cache refresh.
    #[cfg(test)]
    pub(crate) fn browser_row_truncation_frame_counts(&self) -> BrowserRowTruncationFrameCounts {
        self.browser_row_truncation_frame_counts
    }

    /// Update animation clocks by a frame delta using explicit style motion tokens.
    pub(crate) fn tick_with_style(&mut self, delta_seconds: f32, style: &StyleTokens) {
        self.playhead_trail_elapsed_seconds =
            (self.playhead_trail_elapsed_seconds + delta_seconds.max(0.0)).max(0.0);
        if self.needs_animation() {
            let speed = if self.transport_running {
                style.motion_speed_transport
            } else {
                style.motion_speed_idle
            };
            self.pulse_phase =
                (self.pulse_phase + delta_seconds * speed).rem_euclid(std::f32::consts::TAU);
        }
        if let Some(mut flash) = self.waveform_toolbar_flash {
            flash.ticks_remaining = flash.ticks_remaining.saturating_sub(1);
            self.waveform_toolbar_flash = (flash.ticks_remaining > 0).then_some(flash);
        }
        self.waveform_selection_flash_ticks = self.waveform_selection_flash_ticks.saturating_sub(1);
        self.waveform_edit_selection_flash_ticks =
            self.waveform_edit_selection_flash_ticks.saturating_sub(1);
        self.source_add_button_flash_ticks = self.source_add_button_flash_ticks.saturating_sub(1);
        self.status_options_button_flash_ticks =
            self.status_options_button_flash_ticks.saturating_sub(1);
    }

    /// Return the current hover-overlay fingerprint.
    pub(crate) fn hover_overlay_fingerprint(&self) -> HoverOverlayFingerprint {
        HoverOverlayFingerprint {
            hovered: self.hovered,
            hovered_browser_visible_row: self.hovered_browser_visible_row,
            hovered_folder_pane: self.hovered_folder_pane,
            hovered_folder_row_index: self.hovered_folder_row_index,
            hovered_waveform_toolbar_hint: self.hovered_waveform_toolbar_hint,
            browser_search_editor_signature: text_field_visual_signature(
                self.browser_search_editor_visual.as_ref(),
            ),
            folder_create_editor_signature: text_field_visual_signature(
                self.folder_create_editor_visual.as_ref(),
            ),
        }
    }

    /// Return the current focus-overlay fingerprint.
    pub(crate) fn focus_overlay_fingerprint(&self) -> FocusOverlayFingerprint {
        FocusOverlayFingerprint {
            has_focus_emphasis: self.has_focus_emphasis,
        }
    }

    /// Return the current modal-overlay fingerprint.
    pub(crate) fn modal_overlay_fingerprint(&self) -> ModalOverlayFingerprint {
        ModalOverlayFingerprint {
            source_context_menu_pane: self.source_context_menu.map(|menu| menu.pane),
            source_context_menu_row_index: self.source_context_menu.map(|menu| menu.row_index),
            source_context_menu_anchor_x_bits: self
                .source_context_menu
                .map(|menu| menu.anchor.x.to_bits()),
            source_context_menu_anchor_y_bits: self
                .source_context_menu
                .map(|menu| menu.anchor.y.to_bits()),
            browser_context_menu_row_index: self.browser_context_menu.map(|menu| menu.visible_row),
            browser_context_menu_anchor_x_bits: self
                .browser_context_menu
                .map(|menu| menu.anchor.x.to_bits()),
            browser_context_menu_anchor_y_bits: self
                .browser_context_menu
                .map(|menu| menu.anchor.y.to_bits()),
        }
    }
}

fn sync_folder_pane_model(
    pane_state: &mut FolderPaneRuntimeState,
    pane_model: &crate::compat_app_contract::FolderPaneModel,
) {
    if pane_state.last_focused_row != pane_model.focused_tree_row {
        pane_state.last_focused_row = pane_model.focused_tree_row;
        pane_state.autoscroll = true;
        pane_state.cache_key = None;
    }
    if pane_model.tree_rows.is_empty() {
        pane_state.window_start = 0;
    }
}
