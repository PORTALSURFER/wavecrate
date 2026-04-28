//! Waveform toolbar layout and button rendering helpers.

use super::super::*;
use super::{
    waveform_toolbar_icon_rect, waveform_toolbar_overlay_icon_color,
    waveform_toolbar_overlay_icon_rect, waveform_toolbar_visual_color,
};

pub(in crate::gui::native_shell::state) fn waveform_toolbar_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &NativeMotionModel,
    bpm_input_active: bool,
    bpm_input_display: Option<&str>,
) -> Vec<WaveformToolbarButton> {
    let bpm_value_label = waveform_toolbar_bpm_value_label(model, bpm_input_display);
    let (transport_label, transport_icon, transport_action, transport_color) =
        if model.transport_running {
            (
                "Stop",
                Some(WaveformToolbarIcon::Stop),
                Some(UiAction::HandleEscape),
                style.highlight_orange_soft,
            )
        } else {
            (
                "Play",
                Some(WaveformToolbarIcon::Play),
                Some(UiAction::ToggleTransport),
                style.accent_warning,
            )
        };
    let specs = vec![
        (
            "Channel",
            Some(
                if model.waveform_channel_view == crate::app::WaveformChannelViewModel::Stereo {
                    WaveformToolbarIcon::Stereo
                } else {
                    WaveformToolbarIcon::Mono
                },
            ),
            None,
            None,
            true,
            false,
            Some(UiAction::SetWaveformChannelView {
                stereo: model.waveform_channel_view != crate::app::WaveformChannelViewModel::Stereo,
            }),
            style.text_primary,
        ),
        (
            "Norm",
            Some(WaveformToolbarIcon::Normalize),
            None,
            None,
            true,
            model.waveform_normalized_audition_enabled,
            Some(UiAction::SetNormalizedAuditionEnabled {
                enabled: !model.waveform_normalized_audition_enabled,
            }),
            if model.waveform_normalized_audition_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "BPM Value",
            None,
            None,
            Some(bpm_value_label),
            true,
            bpm_input_active,
            None,
            style.text_primary,
        ),
        (
            "BPM Snap",
            Some(WaveformToolbarIcon::BpmSnap),
            None,
            None,
            true,
            model.waveform_bpm_snap_enabled,
            Some(UiAction::SetBpmSnapEnabled {
                enabled: !model.waveform_bpm_snap_enabled,
            }),
            if model.waveform_bpm_snap_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Rel Grid",
            Some(WaveformToolbarIcon::RelativeBpmGrid),
            None,
            None,
            true,
            model.waveform_relative_bpm_grid_enabled,
            Some(UiAction::SetRelativeBpmGridEnabled {
                enabled: !model.waveform_relative_bpm_grid_enabled,
            }),
            if model.waveform_relative_bpm_grid_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Tr Snap",
            Some(WaveformToolbarIcon::TransientSnap),
            None,
            None,
            true,
            model.waveform_transient_snap_enabled,
            Some(UiAction::SetTransientSnapEnabled {
                enabled: !model.waveform_transient_snap_enabled,
            }),
            if model.waveform_transient_snap_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Show Tr",
            Some(WaveformToolbarIcon::ShowTransients),
            None,
            None,
            true,
            model.waveform_transient_markers_enabled,
            Some(UiAction::SetTransientMarkersEnabled {
                enabled: !model.waveform_transient_markers_enabled,
            }),
            if model.waveform_transient_markers_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Slice",
            Some(WaveformToolbarIcon::Slice),
            None,
            None,
            true,
            model.waveform_slice_mode_enabled,
            Some(UiAction::SetSliceModeEnabled {
                enabled: !model.waveform_slice_mode_enabled,
            }),
            if model.waveform_slice_mode_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Silence Split",
            Some(WaveformToolbarIcon::Slice),
            None,
            None,
            model.waveform_loaded_label.is_some() && !model.waveform_loading,
            false,
            Some(UiAction::DetectWaveformSilenceSlices),
            style.highlight_blue_soft,
        ),
        (
            "Exact Dedupe",
            Some(WaveformToolbarIcon::Slice),
            None,
            None,
            model.waveform_loaded_label.is_some() && !model.waveform_loading,
            false,
            Some(UiAction::DetectWaveformExactDuplicateSlices),
            style.highlight_blue_soft,
        ),
        (
            "Clean Dups",
            Some(WaveformToolbarIcon::Slice),
            None,
            None,
            model.waveform_loaded_label.is_some()
                && !model.waveform_loading
                && model.waveform_exact_duplicate_cleanup_available,
            false,
            Some(UiAction::CleanWaveformExactDuplicateSlices),
            style.highlight_cyan_soft,
        ),
        (
            "Loop",
            Some(WaveformToolbarIcon::Loop),
            if model.waveform_loop_lock_enabled {
                Some(WaveformToolbarIcon::Lock)
            } else {
                None
            },
            None,
            true,
            model.waveform_loop_enabled,
            Some(UiAction::ToggleLoopPlayback),
            if model.waveform_loop_enabled && model.waveform_loop_lock_enabled {
                style.accent_warning
            } else if model.waveform_loop_enabled {
                style.accent_warning
            } else {
                style.text_muted
            },
        ),
        (
            "Compare",
            Some(WaveformToolbarIcon::Play),
            None,
            None,
            model.waveform_compare_anchor_available,
            false,
            Some(UiAction::PlayCompareAnchor),
            if model.waveform_compare_anchor_available {
                style.highlight_cyan_soft
            } else {
                style.text_muted
            },
        ),
        (
            transport_label,
            transport_icon,
            None,
            None,
            true,
            model.transport_running,
            transport_action,
            transport_color,
        ),
        (
            "Rec",
            Some(WaveformToolbarIcon::Record),
            None,
            None,
            false,
            false,
            None,
            style.highlight_blue_soft,
        ),
    ];
    let content = WaveformToolbarSurfaceContent {
        items: specs
            .iter()
            .map(
                |(label, _, _, display_text, enabled, active, _, _)| WaveformToolbarSurfaceItem {
                    label: (*label).to_string(),
                    kind: waveform_toolbar_surface_item_kind(label),
                    value: display_text.clone(),
                    enabled: *enabled,
                    active: *active,
                },
            )
            .collect(),
    };
    let surface_layout =
        resolve_waveform_toolbar_surface_layout(layout.waveform_header, style.sizing, &content);
    surface_layout
        .item_rects
        .iter()
        .copied()
        .zip(specs)
        .filter(|(rect, _)| rect.width() > 1.0 && rect.height() > 1.0)
        .map(
            |(
                rect,
                (label, icon, overlay_icon, display_text, enabled, active, action, text_color),
            )| {
                WaveformToolbarButton {
                    rect,
                    label,
                    icon,
                    overlay_icon,
                    display_text,
                    enabled,
                    active,
                    action,
                    text_color,
                }
            },
        )
        .collect()
}

fn waveform_toolbar_surface_item_kind(label: &str) -> WaveformToolbarSurfaceItemKind {
    match label {
        "BPM Value" => WaveformToolbarSurfaceItemKind::TextInput,
        "Channel" | "Norm" | "BPM Snap" | "Rel Grid" | "Tr Snap" | "Show Tr" | "Slice" | "Loop" => {
            WaveformToolbarSurfaceItemKind::Toggle
        }
        _ => WaveformToolbarSurfaceItemKind::Button,
    }
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_bpm_value_label(
    model: &NativeMotionModel,
    bpm_input_display: Option<&str>,
) -> String {
    if let Some(display) = bpm_input_display {
        return display.to_string();
    }
    model
        .waveform_tempo_label
        .as_deref()
        .and_then(crate::app::parse_waveform_tempo_number_text)
        .unwrap_or_else(|| String::from("120.0"))
}

pub(in crate::gui::native_shell::state) fn waveform_toolbar_left_edge(
    buttons: &[WaveformToolbarButton],
    fallback: f32,
) -> f32 {
    buttons
        .iter()
        .map(|button| button.rect.min.x)
        .min_by(f32::total_cmp)
        .unwrap_or(fallback)
}

pub(in crate::gui::native_shell::state) fn render_waveform_toolbar_buttons(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    style: &StyleTokens,
    sizing: SizingTokens,
    buttons: &[WaveformToolbarButton],
    hovered_hint: Option<WaveformToolbarHoverHint>,
    flashed_hint: Option<WaveformToolbarHoverHint>,
    motion_wave: f32,
    hide_active_bpm_value_text: bool,
) {
    for button in buttons {
        if hide_active_bpm_value_text && button.label == "BPM Value" {
            continue;
        }
        let label_rect = compute_action_button_text_rect(button.rect, sizing);
        let button_hint = waveform_toolbar_hover_hint(button.label);
        let is_hovered = button_hint.is_some() && button_hint == hovered_hint;
        let is_flashed = button_hint.is_some() && button_hint == flashed_hint;
        let icon_color = waveform_toolbar_visual_color(
            style,
            button.text_color,
            button.enabled,
            button.active,
            is_hovered,
            is_flashed,
            motion_wave,
        );
        let main_icon_rect =
            waveform_toolbar_icon_rect(button.rect, sizing, button.active, is_hovered, is_flashed);
        let rendered_main_icon = if let Some(icon) = toolbar_icon_for_button(button) {
            emit_toolbar_svg_icon(primitives, icon, main_icon_rect, icon_color)
        } else {
            false
        };
        if !rendered_main_icon {
            emit_text(
                text_runs,
                TextRun {
                    text: button
                        .display_text
                        .clone()
                        .unwrap_or_else(|| button.label.to_string()),
                    position: label_rect.min,
                    font_size: sizing.font_meta,
                    color: icon_color,
                    max_width: Some(label_rect.width().max(12.0)),
                    align: TextAlign::Center,
                },
            );
        }
        if let Some(overlay_icon) = button.overlay_icon {
            let _ = emit_toolbar_svg_icon(
                primitives,
                overlay_icon,
                waveform_toolbar_overlay_icon_rect(button.rect, sizing),
                waveform_toolbar_overlay_icon_color(style, icon_color),
            );
        }
    }
}
