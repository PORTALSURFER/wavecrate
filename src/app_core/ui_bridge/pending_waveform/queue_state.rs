/// Queue of high-frequency waveform actions that can be coalesced per pull frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app_core::ui_bridge) struct PendingWaveformActions {
    /// Monotonic order counter for cross-lane waveform actions in this batch.
    pub(super) action_order: u64,
    /// Order of the latest queued playback-selection endpoint mutation.
    pub(super) selection_order: Option<u64>,
    /// Order of the latest queued waveform zoom mutation.
    pub(super) zoom_order: Option<u64>,
    /// Order of the latest queued waveform center mutation.
    pub(super) view_center_order: Option<u64>,
    /// Latest seek target in normalized nanounits.
    pub(in crate::app_core::ui_bridge) seek_nanos: Option<u32>,
    /// Latest cursor target in normalized nanounits.
    pub(in crate::app_core::ui_bridge) cursor_nanos: Option<u32>,
    /// Latest explicit selection range in normalized micro space.
    pub(in crate::app_core::ui_bridge) selection_range_micros: Option<(u32, u32)>,
    /// Latest explicit selection range in normalized nanounits.
    pub(in crate::app_core::ui_bridge) selection_range_nanos: Option<(u32, u32)>,
    /// Whether the queued selection range should preserve an out-of-bounds view edge.
    pub(in crate::app_core::ui_bridge) selection_preserve_view_edge: bool,
    /// Whether the queued selection range should bypass BPM snapping.
    pub(in crate::app_core::ui_bridge) selection_snap_override: bool,
    /// Whether the queued selection range should recompute BPM from a 4-beat span.
    pub(in crate::app_core::ui_bridge) selection_smart_scale: bool,
    /// Whether selection should be cleared when no range override is queued.
    pub(in crate::app_core::ui_bridge) clear_selection: bool,
    /// Latest waveform viewport center in normalized micro space.
    pub(in crate::app_core::ui_bridge) view_center_micros: Option<u32>,
    /// Latest exact waveform viewport center in normalized nanounits.
    pub(in crate::app_core::ui_bridge) view_center_nanos: Option<u32>,
    /// Net signed waveform zoom step delta accumulated this frame.
    pub(in crate::app_core::ui_bridge) zoom_steps_delta: i16,
    /// Latest queued pointer-anchor ratio for waveform zoom (`0..=1_000_000`).
    pub(in crate::app_core::ui_bridge) zoom_anchor_ratio_micros: Option<u32>,
    /// Whether `ZoomWaveformToSelection` is queued for this frame.
    pub(in crate::app_core::ui_bridge) zoom_to_selection: bool,
    /// Whether `ZoomWaveformFull` is queued for this frame.
    pub(in crate::app_core::ui_bridge) zoom_full: bool,
}

impl PendingWaveformActions {
    pub(super) fn next_action_order(&mut self) -> u64 {
        self.action_order = self.action_order.saturating_add(1);
        self.action_order
    }

    pub(super) fn note_selection_action(&mut self) {
        self.selection_order = Some(self.next_action_order());
    }

    pub(super) fn note_zoom_action(&mut self) {
        self.zoom_order = Some(self.next_action_order());
    }

    pub(super) fn note_view_center_action(&mut self) {
        self.view_center_order = Some(self.next_action_order());
    }

    /// Return true when at least one queued waveform action is present.
    pub(in crate::app_core::ui_bridge) fn has_pending(&self) -> bool {
        self.seek_nanos.is_some()
            || self.cursor_nanos.is_some()
            || self.selection_range_micros.is_some()
            || self.clear_selection
            || self.view_center_micros.is_some()
            || self.view_center_nanos.is_some()
            || self.zoom_steps_delta != 0
            || self.zoom_to_selection
            || self.zoom_full
    }
}
