//! Waveform overlay emission for playhead, selection, loop, and scrollbar chrome.

use super::scrollbar::emit_waveform_scrollbar;
use super::surface::emit_waveform_loading_placeholder;
use super::trail::emit_waveform_playhead_trail;
use super::*;

pub(in crate::gui::native_shell::state) fn push_waveform_playhead_overlay(
    primitives: &mut impl PrimitiveSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    selection_flash_active: bool,
    edit_selection_flash_active: bool,
    selection_flash_tone: WaveformSelectionFlashTone,
    motion_wave: f32,
    playhead_trail_lines: &[PlayheadTrailLine],
    hovered_resize_edge: Option<WaveformResizeHoverEdge>,
) {
    let transport = model.waveform_transport();
    let viewport = model.waveform_viewport();
    let edit_preview = model.waveform_edit_preview();
    let presentation = model.waveform_presentation();
    let image_preview = model.waveform_image_preview();

    if image_preview.loading {
        emit_waveform_loading_placeholder(primitives, layout.waveform_plot, style, motion_wave);
        return;
    }
    emit_waveform_slice_previews(primitives, layout.waveform_plot, style, model);
    let annotations = compute_waveform_annotation_rects_with_nanos(
        layout.waveform_plot,
        style.sizing.border_width,
        transport.selection,
        transport.cursor_milli,
        None,
        viewport.start_micros,
        viewport.end_micros,
        viewport.start_nanos,
        viewport.end_nanos,
    );
    let playhead_rect =
        playhead_marker_rect(layout.waveform_plot, style.sizing.border_width, model);

    if let Some(rect) = annotations.selection {
        let selection_fill = if selection_flash_active {
            let flash_accent = match selection_flash_tone {
                WaveformSelectionFlashTone::Optimistic => style.accent_warning,
                WaveformSelectionFlashTone::Error => style.accent_danger,
            };
            translucent_overlay_color(style.surface_overlay, flash_accent, 0.78)
        } else {
            translucent_overlay_color(style.bg_secondary, style.accent_warning, 0.52)
        };
        let selection_border = if selection_flash_active {
            match selection_flash_tone {
                WaveformSelectionFlashTone::Optimistic => {
                    blend_color(style.accent_warning, style.text_primary, 0.5)
                }
                WaveformSelectionFlashTone::Error => {
                    blend_color(style.accent_danger, style.text_primary, 0.5)
                }
            }
        } else {
            blend_color(style.accent_warning, style.text_primary, 0.28)
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: selection_fill,
            }),
        );
        push_border(
            primitives,
            rect,
            selection_border,
            style.sizing.border_width,
        );
        emit_hovered_selection_resize_edge(
            primitives,
            style,
            rect,
            style.accent_warning,
            hovered_resize_edge,
        );
        if presentation.repeat_enabled {
            emit_waveform_loop_bar(primitives, style, rect);
        }
        emit_selection_shift_handle(primitives, style, rect, style.accent_warning);
        emit_selection_drag_handle(primitives, style, rect);
    }

    if let Some(edit_selection) = edit_preview.selection {
        let edit_selection_rect = compute_waveform_annotation_rects_with_nanos(
            layout.waveform_plot,
            style.sizing.border_width,
            Some(edit_selection),
            None,
            None,
            viewport.start_micros,
            viewport.end_micros,
            viewport.start_nanos,
            viewport.end_nanos,
        )
        .selection;
        if let Some(rect) = edit_selection_rect {
            let edit_fill = if edit_selection_flash_active {
                translucent_overlay_color(style.surface_overlay, style.highlight_blue, 0.82)
            } else {
                translucent_overlay_color(style.bg_secondary, style.highlight_blue, 0.5)
            };
            let edit_border = if edit_selection_flash_active {
                blend_color(style.highlight_blue, style.text_primary, 0.5)
            } else {
                blend_color(style.highlight_blue, style.text_primary, 0.24)
            };
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect,
                    color: edit_fill,
                }),
            );
            push_border(primitives, rect, edit_border, style.sizing.border_width);
            emit_edit_fade_overlays(
                primitives,
                style,
                layout.waveform_plot,
                rect,
                edit_selection,
                edit_preview.leading_end_milli,
                edit_preview.leading_end_micros,
                edit_preview.leading_inner_start_milli,
                edit_preview.leading_inner_start_micros,
                edit_preview.leading_curve_milli,
                edit_preview.trailing_start_milli,
                edit_preview.trailing_start_micros,
                edit_preview.trailing_inner_end_milli,
                edit_preview.trailing_inner_end_micros,
                edit_preview.trailing_curve_milli,
                viewport.start_micros,
                viewport.end_micros,
                style.highlight_blue,
            );
            emit_hovered_edit_resize_edge(
                primitives,
                style,
                rect,
                style.highlight_blue,
                hovered_resize_edge,
            );
            emit_selection_shift_handle(primitives, style, rect, style.highlight_blue);
        }
    }

    if let Some(rect) = annotations.cursor {
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.accent_warning,
            }),
        );
    }
    if let Some(rect) = playhead_rect {
        emit_waveform_playhead_trail(
            primitives,
            layout.waveform_plot,
            style,
            style.sizing.border_width,
            playhead_trail_lines,
            viewport.start_micros,
            viewport.end_micros,
            viewport.start_nanos,
            viewport.end_nanos,
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect,
                color: style.accent_copper,
            }),
        );
    }
    emit_waveform_scrollbar(primitives, layout.waveform_scrollbar_lane, style, model);
}
