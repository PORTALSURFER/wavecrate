//! Coalesced high-frequency waveform action batching helpers.

use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::controller_state::DirtyReason;

/// Toggle immediate application of waveform overlay preview actions.
const IMMEDIATE_WAVEFORM_PREVIEW_ENV: &str = "SEMPAL_NATIVE_BRIDGE_IMMEDIATE_WAVEFORM_PREVIEW";
/// Default mode for immediate waveform overlay preview actions.
const IMMEDIATE_WAVEFORM_PREVIEW_DEFAULT: bool = true;
/// Cached immediate-waveform-preview mode resolved from environment.
static IMMEDIATE_WAVEFORM_PREVIEW_ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
/// Maximum consecutive local-only model pulls before forcing one full prep pass.
pub(super) const LOCAL_MODEL_PULL_FAST_PATH_BURST_LIMIT: u8 = 8;

/// Resolve whether waveform preview actions should apply immediately.
pub(super) fn immediate_waveform_preview_enabled() -> bool {
    *IMMEDIATE_WAVEFORM_PREVIEW_ENABLED.get_or_init(|| {
        std::env::var(IMMEDIATE_WAVEFORM_PREVIEW_ENV)
            .ok()
            .map_or(IMMEDIATE_WAVEFORM_PREVIEW_DEFAULT, |value| {
                crate::env_flags::is_truthy(&value)
            })
    })
}

/// One-shot preparation mode for the next app-model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PendingModelPullPreparation {
    /// Run the normal full pull-preparation path.
    #[default]
    Full,
    /// Skip full controller prep once and project directly from current UI state.
    LocalOnly,
}

/// Queue of high-frequency waveform actions that can be coalesced per pull frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PendingWaveformActions {
    /// Latest seek target in normalized milli space.
    pub(super) seek_milli: Option<u16>,
    /// Latest cursor target in normalized milli space.
    pub(super) cursor_milli: Option<u16>,
    /// Latest explicit selection range in normalized milli space.
    pub(super) selection_range_micros: Option<(u32, u32)>,
    /// Whether the queued selection range should preserve an out-of-bounds view edge.
    pub(super) selection_preserve_view_edge: bool,
    /// Whether the queued selection range should recompute BPM from a 4-beat span.
    pub(super) selection_smart_scale: bool,
    /// Whether selection should be cleared when no range override is queued.
    pub(super) clear_selection: bool,
    /// Net signed waveform zoom step delta accumulated this frame.
    pub(super) zoom_steps_delta: i16,
    /// Latest queued pointer-anchor ratio for waveform zoom (`0..=1_000_000`).
    pub(super) zoom_anchor_ratio_micros: Option<u32>,
    /// Whether `ZoomWaveformToSelection` is queued for this frame.
    pub(super) zoom_to_selection: bool,
    /// Whether `ZoomWaveformFull` is queued for this frame.
    pub(super) zoom_full: bool,
}

impl PendingWaveformActions {
    /// Return true when at least one queued waveform action is present.
    pub(super) fn has_pending(&self) -> bool {
        self.seek_milli.is_some()
            || self.cursor_milli.is_some()
            || self.selection_range_micros.is_some()
            || self.clear_selection
            || self.zoom_steps_delta != 0
            || self.zoom_to_selection
            || self.zoom_full
    }

    /// Queue a coalescable waveform action and return true when absorbed.
    pub(super) fn enqueue(&mut self, action: &NativeUiAction) -> bool {
        match action {
            NativeUiAction::SeekWaveform { position_milli } => {
                self.seek_milli = Some(*position_milli);
                true
            }
            NativeUiAction::SetWaveformCursor { position_milli } => {
                self.cursor_milli = Some(*position_milli);
                true
            }
            NativeUiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => {
                self.selection_range_micros = Some((*start_micros, *end_micros));
                self.selection_preserve_view_edge = *preserve_view_edge;
                self.selection_smart_scale = false;
                self.clear_selection = false;
                true
            }
            NativeUiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => {
                self.selection_range_micros = Some((*start_micros, *end_micros));
                self.selection_preserve_view_edge = false;
                self.selection_smart_scale = true;
                self.clear_selection = false;
                true
            }
            NativeUiAction::ClearWaveformSelection => {
                self.selection_range_micros = None;
                self.selection_preserve_view_edge = false;
                self.selection_smart_scale = false;
                self.clear_selection = true;
                true
            }
            NativeUiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => {
                if self.zoom_full || self.zoom_to_selection {
                    return true;
                }
                let signed_steps = if *zoom_in {
                    i16::from(*steps)
                } else {
                    -i16::from(*steps)
                };
                self.zoom_steps_delta = self.zoom_steps_delta.saturating_add(signed_steps);
                self.zoom_anchor_ratio_micros = if self.zoom_steps_delta == 0 {
                    None
                } else {
                    anchor_ratio_micros.map(|micros| micros.min(1_000_000))
                };
                true
            }
            NativeUiAction::ZoomWaveformToSelection => {
                self.zoom_steps_delta = 0;
                self.zoom_anchor_ratio_micros = None;
                self.zoom_to_selection = true;
                self.zoom_full = false;
                true
            }
            NativeUiAction::ZoomWaveformFull => {
                self.zoom_steps_delta = 0;
                self.zoom_anchor_ratio_micros = None;
                self.zoom_to_selection = false;
                self.zoom_full = true;
                true
            }
            _ => false,
        }
    }

    /// Return the derived-graph dirty reason represented by this pending batch.
    pub(super) fn dirty_reason(&self) -> DirtyReason {
        if self.zoom_full
            || self.zoom_to_selection
            || self.zoom_steps_delta != 0
            || self.selection_smart_scale
        {
            DirtyReason::WaveformViewAction
        } else {
            DirtyReason::WaveformOverlayAction
        }
    }

    /// Return the cursor update after removing redundant cursor+seek pairs.
    ///
    /// A queued seek already updates cursor position, so sending both actions at
    /// the same normalized milli target adds no behavior but does add apply cost.
    pub(super) fn deduped_cursor_milli(&self) -> Option<u16> {
        if self.cursor_milli.is_some() && self.cursor_milli == self.seek_milli {
            None
        } else {
            self.cursor_milli
        }
    }

    /// Build the highest-priority zoom action for this pending batch, if any.
    pub(super) fn zoom_action(&self) -> Option<NativeUiAction> {
        if self.zoom_full {
            return Some(NativeUiAction::ZoomWaveformFull);
        }
        if self.zoom_to_selection {
            return Some(NativeUiAction::ZoomWaveformToSelection);
        }
        if self.zoom_steps_delta == 0 {
            return None;
        }
        let zoom_in = self.zoom_steps_delta.is_positive();
        let steps = self.zoom_steps_delta.unsigned_abs().min(u16::from(u8::MAX)) as u8;
        Some(NativeUiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros: self.zoom_anchor_ratio_micros,
        })
    }

    /// Build the highest-priority selection action for this pending batch, if any.
    pub(super) fn selection_action(&self) -> Option<NativeUiAction> {
        if let Some((start_micros, end_micros)) = self.selection_range_micros {
            return Some(if self.selection_smart_scale {
                NativeUiAction::SetWaveformSelectionRangeSmartScale {
                    start_micros,
                    end_micros,
                }
            } else {
                NativeUiAction::SetWaveformSelectionRange {
                    start_micros,
                    end_micros,
                    preserve_view_edge: self.selection_preserve_view_edge,
                }
            });
        }
        self.clear_selection
            .then_some(NativeUiAction::ClearWaveformSelection)
    }

    /// Emit queued waveform actions in deterministic application order.
    pub(super) fn emit_actions(&self, mut emit: impl FnMut(NativeUiAction)) -> u64 {
        let mut emitted_actions = 0u64;
        if let Some(action) = self.zoom_action() {
            emit(action);
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(action) = self.selection_action() {
            emit(action);
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(position_milli) = self.deduped_cursor_milli() {
            emit(NativeUiAction::SetWaveformCursor { position_milli });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(position_milli) = self.seek_milli {
            emit(NativeUiAction::SeekWaveform { position_milli });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        emitted_actions
    }

    /// Return true when queued actions mutate waveform static rendering content.
    ///
    /// Zoom actions change the waveform viewport and image payload, so native
    /// runtime must pull a full projected model instead of motion-only state.
    pub(super) fn requires_full_model_pull(&self) -> bool {
        self.zoom_steps_delta != 0 || self.zoom_to_selection || self.zoom_full
    }
}
