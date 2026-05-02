//! Motion-driven overlay assembly for native shell state.

use super::*;

#[path = "motion_overlay/playhead_trail.rs"]
mod playhead_trail;

impl NativeShellState {
    /// Build only waveform cursor/playhead motion overlays into reusable buffers.
    pub(crate) fn build_waveform_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(self.pulse_phase);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let playhead_trail_lines = self.update_playhead_trail(layout.waveform_plot, model);
        push_waveform_playhead_overlay(
            primitives,
            layout,
            style,
            model,
            self.waveform_selection_flash_ticks > 0,
            self.waveform_edit_selection_flash_ticks > 0,
            self.waveform_selection_flash_tone,
            motion_wave,
            &playhead_trail_lines,
            self.hovered_waveform_resize_edge,
        );
        if let Some(hover_x) = self.waveform_hover_x {
            // Keep hover preview cursor visually obvious against dense waveform content.
            let hover_marker_width = (sizing.border_width * 2.0).max(2.0);
            if let Some(rect) =
                waveform_hover_marker_rect(layout.waveform_plot, hover_marker_width, hover_x)
            {
                emit_primitive(
                    primitives,
                    Primitive::Rect(FillRect {
                        rect,
                        color: blend_color(style.accent_warning, style.text_primary, 0.72),
                    }),
                );
                push_border(
                    primitives,
                    rect,
                    blend_color(style.accent_warning, style.text_primary, 0.48),
                    sizing.border_width,
                );
            }
        }
        frame.clear_color = style.clear_color;
    }

    /// Build only heavier motion-driven chrome overlays into reusable buffers.
    pub(crate) fn build_chrome_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(self.pulse_phase);
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;

        let waveform_toolbar_buttons = waveform_toolbar_buttons(
            layout,
            style,
            model,
            self.waveform_bpm_input_active,
            self.waveform_bpm_input_display.as_deref(),
        );
        let waveform_toolbar_left = waveform_toolbar_left_edge(
            &waveform_toolbar_buttons,
            layout.waveform_header.max.x - sizing.text_inset_x,
        );
        push_waveform_header_overlay(
            primitives,
            text_runs,
            layout,
            style,
            model,
            Some(waveform_toolbar_left - sizing.action_button_gap),
        );
        if self.waveform_bpm_input_active {
            if let Some(bpm_input_rect) = waveform_toolbar_buttons
                .iter()
                .find(|button| button.label == "BPM Value")
                .map(|button| button.rect)
            {
                if let Some(visual) = self.waveform_bpm_editor_visual.as_ref() {
                    let bpm_text_rect = waveform_toolbar_buttons
                        .iter()
                        .find(|button| button.label == "BPM Value")
                        .map(|button| compute_action_button_text_rect(button.rect, sizing))
                        .unwrap_or(bpm_input_rect);
                    render_active_waveform_bpm_editor(
                        primitives,
                        text_runs,
                        style,
                        sizing,
                        bpm_input_rect,
                        bpm_text_rect,
                        visual,
                    );
                } else {
                    render_waveform_bpm_input_focus_overlay(
                        primitives,
                        style,
                        sizing,
                        bpm_input_rect,
                        motion_wave,
                    );
                }
            }
        }
        render_waveform_toolbar_buttons(
            primitives,
            text_runs,
            style,
            sizing,
            &waveform_toolbar_buttons,
            self.hovered_waveform_toolbar_hint,
            self.waveform_toolbar_flash.map(|flash| flash.hint),
            motion_wave,
            self.waveform_bpm_editor_visual.is_some(),
        );
        if let Some(search_field_rect) = self
            .browser_toolbar_layout
            .as_ref()
            .map(|toolbar| toolbar.search_field)
            .filter(|rect| rect.width() > 1.0)
        {
            if self.hovered_browser_search_field && self.browser_search_editor_visual.is_none() {
                render_browser_search_field_hover_overlay(
                    primitives,
                    style,
                    sizing,
                    search_field_rect,
                    motion_wave,
                );
            }
        }
        if let Some((chip_rect, rating_level)) =
            self.browser_toolbar_layout.as_ref().and_then(|toolbar| {
                let hovered_level = self.hovered_browser_rating_filter_level?;
                let index = browser_rating_filter_chip_index(hovered_level)?;
                let chip_rect = toolbar.rating_filter_chips[index];
                (chip_rect.width() > 1.0).then_some((chip_rect, hovered_level))
            })
        {
            let active = browser_rating_filter_chip_index(rating_level)
                .and_then(|index| model.active_rating_filters.get(index))
                .copied()
                .unwrap_or(false);
            render_browser_rating_filter_chip_hover_overlay(
                primitives,
                style,
                sizing,
                chip_rect,
                rating_level,
                active,
                motion_wave,
            );
        }
        if let Some((chip_rect, chip)) = self.browser_toolbar_layout.as_ref().and_then(|toolbar| {
            let hovered_chip = self.hovered_browser_playback_age_filter_chip?;
            let index = browser_playback_age_filter_chip_index(hovered_chip)?;
            let chip_rect = toolbar.playback_age_filter_chips[index];
            (chip_rect.width() > 1.0).then_some((chip_rect, hovered_chip))
        }) {
            let active = browser_playback_age_filter_chip_index(chip)
                .and_then(|index| model.active_playback_age_filters.get(index))
                .copied()
                .unwrap_or(false);
            render_browser_playback_age_filter_chip_hover_overlay(
                primitives,
                style,
                sizing,
                chip_rect,
                chip,
                active,
                motion_wave,
            );
        }
        if let Some(chip_rect) = self
            .browser_toolbar_layout
            .as_ref()
            .map(|toolbar| toolbar.marked_filter_chip)
            .filter(|rect| rect.width() > 1.0)
            .filter(|_| self.hovered_browser_marked_filter)
        {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: chip_rect,
                    color: browser_marked_filter_chip_hover_fill(
                        style,
                        model.marked_filter_active,
                        motion_wave,
                    ),
                }),
            );
            push_border(
                primitives,
                chip_rect,
                browser_marked_filter_chip_hover_border(
                    style,
                    model.marked_filter_active,
                    motion_wave,
                ),
                sizing.border_width,
            );
        }
        if let Some(button_rect) = source_add_button_rect(layout.sidebar_header, sizing) {
            let hovered = self.hovered_source_add_button;
            let flashed = self.source_add_button_flash_ticks > 0;
            if hovered || flashed {
                render_source_add_button_overlay(
                    primitives,
                    text_runs,
                    style,
                    sizing,
                    button_rect,
                    hovered,
                    flashed,
                    motion_wave,
                );
            }
        }
        if let Some(button_rect) = top_bar_options_button_rect(layout.top_bar, sizing) {
            let hovered = self.hovered_status_options_button;
            let flashed = self.status_options_button_flash_ticks > 0;
            if hovered || flashed {
                render_status_options_button(
                    primitives,
                    style,
                    sizing,
                    button_rect,
                    "",
                    self.status_options_button_error,
                    hovered,
                    flashed,
                    motion_wave,
                );
            }
        }

        let tabs = resolve_browser_tabs_surface_layout(
            layout.browser_tabs,
            sizing,
            &BrowserTabsSurfaceContent {
                items_label: String::new(),
                map_label: String::new(),
            },
        );
        let (samples_fill, map_fill) = if !model.map_active {
            (
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
                style.surface_base,
            )
        } else {
            (
                style.surface_base,
                blend_color(
                    style.surface_overlay,
                    style.bg_tertiary,
                    style.state_selected_blend + (motion_wave * 0.1),
                ),
            )
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: tabs.items,
                color: samples_fill,
            }),
        );
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: tabs.map,
                color: map_fill,
            }),
        );
        push_border(primitives, tabs.items, style.border, sizing.border_width);
        push_border(
            primitives,
            tabs.map,
            blend_color(style.accent_mint, style.text_primary, 0.42),
            sizing.border_width,
        );
        Self::push_status_right_motion_overlay(
            primitives,
            text_runs,
            layout,
            style,
            &model.status_right,
            None,
        );

        frame.clear_color = style.clear_color;
    }

    /// Build all motion-sensitive overlays into one reusable buffer.
    #[cfg(test)]
    pub(crate) fn build_motion_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &NativeMotionModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_waveform_motion_overlay_into(layout, style, model, frame);
        let mut chrome_frame = NativeViewFrame {
            clear_color: style.clear_color,
            primitives: Vec::new(),
            text_runs: Vec::new(),
        };
        self.build_chrome_motion_overlay_into(layout, style, model, &mut chrome_frame);
        frame.primitives.extend(chrome_frame.primitives);
        frame.text_runs.extend(chrome_frame.text_runs);
        frame.clear_color = style.clear_color;
    }

    fn push_status_right_motion_overlay(
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        layout: &ShellLayout,
        style: &StyleTokens,
        status_right: &str,
        options_button_rect: Option<Rect>,
    ) {
        if status_right.is_empty() {
            return;
        }
        let sizing = style.sizing;
        let text_segment = if let Some(button_rect) = options_button_rect {
            Rect::from_min_max(
                layout.status_right_segment.min,
                Point::new(
                    (button_rect.min.x - sizing.text_inset_x.max(3.0))
                        .max(layout.status_right_segment.min.x),
                    layout.status_right_segment.max.y,
                ),
            )
        } else {
            layout.status_right_segment
        };
        let background_rect = status_motion_overlay_rect(text_segment, sizing.border_width);
        if background_rect.width() > 0.0 && background_rect.height() > 0.0 {
            emit_primitive(
                primitives,
                Primitive::Rect(FillRect {
                    rect: background_rect,
                    color: style.surface_raised,
                }),
            );
        }
        let status_text_rect =
            status_right_text_rect(layout.status_right_segment, sizing, options_button_rect);
        emit_text(
            text_runs,
            TextRun {
                text: truncate_to_width(
                    status_right,
                    status_text_rect.width().max(36.0),
                    sizing.font_status,
                ),
                position: status_text_rect.min,
                font_size: sizing.font_status,
                color: style.text_muted,
                max_width: Some(status_text_rect.width().max(36.0)),
                align: TextAlign::Right,
            },
        );
    }
}

pub(in crate::gui::native_shell::state) fn status_motion_overlay_rect(
    segment: Rect,
    stroke: f32,
) -> Rect {
    let inset = stroke.max(1.0);
    let min = Point::new(
        (segment.min.x + inset).min(segment.max.x),
        (segment.min.y + inset).min(segment.max.y),
    );
    let max = Point::new(
        (segment.max.x - inset).max(min.x),
        (segment.max.y - inset).max(min.y),
    );
    Rect::from_min_max(min, max)
}
