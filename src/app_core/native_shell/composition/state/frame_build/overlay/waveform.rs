use super::*;

pub(super) fn push_waveform_toolbar_hover_tooltip(
    primitives: &mut impl PrimitiveSink,
    text_runs: &mut impl TextRunSink,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    shell_state: &mut NativeShellState,
) {
    let Some(hint) = shell_state.hovered_waveform_toolbar_hint else {
        return;
    };
    let motion_model = NativeMotionModel::from_app_model(model);
    let buttons = shell_state.cached_waveform_toolbar_buttons(layout, style, &motion_model);
    let Some(button_rect) = buttons
        .iter()
        .find(|button| waveform_toolbar_hover_hint(button.label) == Some(hint))
        .map(|button| button.rect)
    else {
        return;
    };
    let text = waveform_toolbar_hover_hint_text(hint, model);
    let font_size = style.sizing.font_status.max(9.0);
    let text_padding_x = (font_size * 0.55).max(4.0);
    let text_padding_y = (font_size * 0.35).max(3.0);
    let tooltip_width = ((text.chars().count() as f32 * font_size * 0.54) + (text_padding_x * 2.0))
        .clamp(84.0, layout.waveform_card.width().max(84.0));
    let tooltip_height = (font_size + (text_padding_y * 2.0)).max(16.0);
    let gap = style.sizing.border_width.max(1.0) * 3.0;
    let mut min_x = ((button_rect.min.x + button_rect.max.x) * 0.5) - (tooltip_width * 0.5);
    min_x = min_x.clamp(
        layout.waveform_card.min.x + style.sizing.text_inset_x,
        layout.waveform_card.max.x - style.sizing.text_inset_x - tooltip_width,
    );
    let preferred_top = button_rect.min.y - gap - tooltip_height;
    let min_y = if preferred_top >= layout.waveform_card.min.y + style.sizing.border_width {
        preferred_top
    } else {
        (button_rect.max.y + gap).min(layout.waveform_card.max.y - tooltip_height)
    };
    let tooltip_rect = Rect::from_min_max(
        Point::new(min_x, min_y),
        Point::new(min_x + tooltip_width, min_y + tooltip_height),
    );
    emit_primitive(
        primitives,
        Primitive::Rect(FillRect {
            rect: tooltip_rect,
            color: blend_color(style.surface_overlay, style.bg_tertiary, 0.76),
        }),
    );
    push_border(
        primitives,
        tooltip_rect,
        blend_color(style.border_emphasis, style.text_primary, 0.58),
        style.sizing.border_width,
    );
    emit_text(
        text_runs,
        TextRun {
            text,
            position: Point::new(
                tooltip_rect.min.x + text_padding_x,
                tooltip_rect.min.y + text_padding_y,
            ),
            font_size,
            color: style.text_primary,
            max_width: Some((tooltip_rect.width() - (text_padding_x * 2.0)).max(16.0)),
            align: TextAlign::Left,
        },
    );
}

fn waveform_toolbar_hover_hint_text(hint: WaveformToolbarHoverHint, model: &AppModel) -> String {
    match hint {
        WaveformToolbarHoverHint::ChannelView => {
            if model.waveform_chrome.channel_view == crate::app::WaveformChannelViewModel::Stereo {
                String::from("Switch waveform view to mono")
            } else {
                String::from("Switch waveform view to split stereo")
            }
        }
        WaveformToolbarHoverHint::NormalizedAudition => {
            if model.waveform_chrome.normalized_audition_enabled {
                String::from("Disable normalized audition")
            } else {
                String::from("Enable normalized audition")
            }
        }
        WaveformToolbarHoverHint::BpmValue => model
            .waveform
            .tempo_label
            .as_deref()
            .map(|tempo| format!("Edit playback BPM ({tempo})"))
            .unwrap_or_else(|| String::from("Edit playback BPM")),
        WaveformToolbarHoverHint::BpmSnap => {
            if model.waveform_chrome.bpm_snap_enabled {
                String::from("Disable BPM snapping")
            } else {
                String::from("Enable BPM snapping")
            }
        }
        WaveformToolbarHoverHint::RelativeBpmGrid => {
            if model.waveform_chrome.relative_bpm_grid_enabled {
                String::from("Use sample-start BPM grid")
            } else {
                String::from("Use selection-relative BPM grid")
            }
        }
        WaveformToolbarHoverHint::TransientSnap => {
            if model.waveform_chrome.transient_snap_enabled {
                String::from("Disable transient snapping")
            } else {
                String::from("Enable transient snapping")
            }
        }
        WaveformToolbarHoverHint::ShowTransients => {
            if model.waveform_chrome.transient_markers_enabled {
                String::from("Hide transient markers")
            } else {
                String::from("Show transient markers")
            }
        }
        WaveformToolbarHoverHint::SliceMode => {
            if model.waveform_chrome.slice_mode_enabled {
                String::from("Disable slice mode")
            } else {
                String::from("Enable slice mode")
            }
        }
        WaveformToolbarHoverHint::SilenceSplit => {
            String::from("Detect silence-based waveform slices")
        }
        WaveformToolbarHoverHint::ExactDedupe => String::from(
            "Scan the waveform for near-duplicate hit windows using the current selection size",
        ),
        WaveformToolbarHoverHint::CleanDuplicates => String::from(
            "Remove marked duplicate windows and keep the first copy plus any right-click keeps",
        ),
        WaveformToolbarHoverHint::Loop => {
            if model.waveform_chrome.loop_lock_enabled && model.waveform.loop_enabled {
                String::from("Loop locked on. Click to unlock and disable; Shift-click locks off")
            } else if model.waveform_chrome.loop_lock_enabled {
                String::from("Loop locked off. Click to unlock and enable; Shift-click locks on")
            } else if model.waveform.loop_enabled {
                String::from("Disable loop playback")
            } else {
                String::from("Enable loop playback")
            }
        }
        WaveformToolbarHoverHint::Compare => model
            .waveform_chrome
            .compare_anchor_label
            .as_deref()
            .map(|label| format!("Play compare anchor ({label})"))
            .unwrap_or_else(|| String::from("Set a compare anchor to enable compare playback")),
        WaveformToolbarHoverHint::Stop => String::from("Stop playback"),
        WaveformToolbarHoverHint::Play => {
            if model.transport_running {
                String::from("Pause playback")
            } else {
                String::from("Start playback")
            }
        }
        WaveformToolbarHoverHint::Record => String::from("Recording is currently unavailable"),
    }
}
