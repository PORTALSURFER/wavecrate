//! Waveform overlay emission for playhead, selection, loop, and scrollbar chrome.

mod edit;
mod selection;

use super::scrollbar::emit_waveform_scrollbar;
use super::surface::emit_waveform_loading_placeholder;
use super::trail::emit_waveform_playhead_trail;
use super::*;

use self::{
    edit::emit_waveform_edit_selection,
    selection::{WaveformSelectionOverlay, emit_waveform_selection},
};

pub(in crate::app_core::native_shell::composition::state) struct WaveformOverlayFlashes {
    pub(in crate::app_core::native_shell::composition::state) selection_active: bool,
    pub(in crate::app_core::native_shell::composition::state) edit_selection_active: bool,
    pub(in crate::app_core::native_shell::composition::state) selection_tone:
        WaveformSelectionFlashTone,
}

pub(in crate::app_core::native_shell::composition::state) struct WaveformOverlayInput<'a> {
    pub(in crate::app_core::native_shell::composition::state) layout: &'a ShellLayout,
    pub(in crate::app_core::native_shell::composition::state) style: &'a StyleTokens,
    pub(in crate::app_core::native_shell::composition::state) model: &'a NativeMotionModel,
    pub(in crate::app_core::native_shell::composition::state) flashes: WaveformOverlayFlashes,
    pub(in crate::app_core::native_shell::composition::state) motion_wave: f32,
    pub(in crate::app_core::native_shell::composition::state) playhead_trail_lines:
        &'a [PlayheadTrailLine],
    pub(in crate::app_core::native_shell::composition::state) hovered_resize_edge:
        Option<WaveformResizeHoverEdge>,
}

pub(in crate::app_core::native_shell::composition::state) fn push_waveform_playhead_overlay(
    primitives: &mut impl PrimitiveSink,
    input: WaveformOverlayInput<'_>,
) {
    let transport = input.model.waveform_transport();
    let viewport = input.model.waveform_viewport();
    let image_preview = input.model.waveform_image_preview();

    if image_preview.loading {
        emit_waveform_loading_placeholder(
            primitives,
            input.layout.waveform_plot,
            input.style,
            input.motion_wave,
        );
        return;
    }
    emit_waveform_slice_previews(
        primitives,
        input.layout.waveform_plot,
        input.style,
        input.model,
    );
    let annotations = compute_waveform_annotation_rects_with_nanos(
        input.layout.waveform_plot,
        input.style.sizing.border_width,
        transport.selection,
        transport.cursor_milli,
        None,
        viewport.start_micros,
        viewport.end_micros,
        viewport.start_nanos,
        viewport.end_nanos,
    );
    let playhead_rect = playhead_marker_rect(
        input.layout.waveform_plot,
        input.style.sizing.border_width,
        input.model,
    );

    if let Some(rect) = annotations.selection {
        emit_waveform_selection(
            primitives,
            WaveformSelectionOverlay {
                style: input.style,
                rect,
                flashes: &input.flashes,
                repeat_enabled: input.model.waveform_presentation().repeat_enabled,
                hovered_resize_edge: input.hovered_resize_edge,
            },
        );
    }

    emit_waveform_edit_selection(primitives, &input, &viewport);

    if let Some(rect) = annotations.cursor {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: input.style.accent_warning,
            }),
        );
    }
    if let Some(rect) = playhead_rect {
        emit_waveform_playhead_trail(
            primitives,
            input.layout.waveform_plot,
            input.style,
            input.style.sizing.border_width,
            input.playhead_trail_lines,
            viewport.start_micros,
            viewport.end_micros,
            viewport.start_nanos,
            viewport.end_nanos,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: input.style.accent_copper,
            }),
        );
    }
    emit_waveform_scrollbar(
        primitives,
        input.layout.waveform_scrollbar_lane,
        input.style,
        input.model,
    );
}
