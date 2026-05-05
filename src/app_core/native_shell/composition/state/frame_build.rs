//! Core static-frame and state-overlay builders extracted from native shell state.

use super::*;

mod browser;
mod chrome;
mod map;
mod overlay;
mod status_bar;
mod waveform;

use self::{browser::*, chrome::*, map::*, overlay::*, status_bar::*, waveform::*};

struct StaticFrameCtx<'a> {
    layout: &'a ShellLayout,
    style: &'a StyleTokens,
    model: &'a AppModel,
    sizing: SizingTokens,
    motion_wave: f32,
}

impl NativeShellState {
    pub(super) fn build_frame_with_style_into_with_motion_sinks(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        primitives: &mut impl PrimitiveSink,
        text_runs: &mut impl TextRunSink,
        pulse_phase: f32,
        include_overlays: bool,
        motion_model: Option<&NativeMotionModel>,
        static_segment_filter: Option<StaticFrameSegment>,
    ) {
        let sizing = style.sizing;
        let motion_wave = interaction_wave(pulse_phase);
        let ctx = StaticFrameCtx {
            layout,
            style,
            model,
            sizing,
            motion_wave,
        };
        let build_global_static =
            static_segment_matches(static_segment_filter, StaticFrameSegment::GlobalStatic);
        let build_waveform_overlay =
            static_segment_matches(static_segment_filter, StaticFrameSegment::WaveformOverlay);
        let build_browser_rows_window =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserRowsWindow);
        let build_map_panel =
            static_segment_matches(static_segment_filter, StaticFrameSegment::MapPanel);
        let build_browser_frame =
            static_segment_matches(static_segment_filter, StaticFrameSegment::BrowserFrame);
        let build_status_bar =
            static_segment_matches(static_segment_filter, StaticFrameSegment::StatusBar);
        let build_browser_rows_or_map = build_browser_rows_window || build_map_panel;
        if build_browser_frame {
            self.browser_segment_text_frame_counts = SegmentTextCacheFrameCounts::default();
        }
        if build_status_bar {
            self.status_bar_text_frame_counts = SegmentTextCacheFrameCounts::default();
        }

        render_static_shell_surfaces(&ctx, primitives);

        if build_waveform_overlay {
            render_waveform_static(self, &ctx, primitives, text_runs, motion_model);
        }

        if build_browser_rows_or_map {
            if model.map.active && build_map_panel {
                render_map_panel(&ctx, primitives);
            } else if !model.map.active && build_browser_rows_window {
                render_browser_rows_window(self, &ctx, primitives, text_runs);
            }
        }

        render_shell_borders(&ctx, primitives);

        if build_global_static {
            render_top_bar_controls(self, &ctx, primitives, text_runs);
        }
        if build_browser_frame {
            render_browser_frame(self, &ctx, primitives, text_runs);
        }
        if build_global_static {
            render_sidebar(self, &ctx, primitives, text_runs);
        }
        // Waveform summary text is produced during overlay rendering so it can
        // update while transport advances without invalidating the static scene.
        if model.map.active && build_map_panel {
            render_map_header(&ctx, text_runs);
        } else if build_browser_frame {
            render_browser_table_header(&ctx, primitives, text_runs);
        }
        if build_browser_frame {
            render_browser_footer(self, &ctx, text_runs);
        }

        if build_status_bar {
            render_status_bar(self, layout, style, model, primitives, text_runs);
        }

        if include_overlays {
            render_modal_overlays(primitives, text_runs, layout, style, model);
        }
    }

    /// Build only state-driven overlays into reusable buffers.
    #[cfg(test)]
    pub(crate) fn build_state_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;
        render_state_overlay(self, layout, style, model, primitives, text_runs);

        frame.clear_color = style.clear_color;
    }

    /// Build only hover/editor overlay primitives into reusable buffers.
    pub(crate) fn build_hover_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;
        render_hover_overlay(self, layout, style, model, primitives, text_runs);

        frame.clear_color = style.clear_color;
    }

    /// Build only focus-emphasis overlay primitives into reusable buffers.
    pub(crate) fn build_focus_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;
        render_focus_overlay(self, layout, style, model, primitives, text_runs);

        frame.clear_color = style.clear_color;
    }

    /// Build only modal and popover overlay primitives into reusable buffers.
    pub(crate) fn build_modal_overlay_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        frame.primitives.clear();
        frame.text_runs.clear();
        let primitives = &mut frame.primitives;
        let text_runs = &mut frame.text_runs;
        render_modal_overlay(self, layout, style, model, primitives, text_runs);

        frame.clear_color = style.clear_color;
    }
}
