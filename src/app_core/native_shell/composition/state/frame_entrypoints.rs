use super::*;
use std::cell::RefCell;

impl NativeShellState {
    /// Build a native frame from state + layout + style tokens.
    #[cfg(test)]
    pub(crate) fn build_frame_with_style(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
    ) -> NativeViewFrame {
        let mut frame = NativeViewFrame {
            clear_color: style.clear_color,
            primitives: Vec::new(),
            text_runs: Vec::new(),
        };
        self.build_frame_with_style_into(layout, style, model, &mut frame);
        frame
    }

    /// Build a native frame from state + layout + style tokens into reusable buffers.
    #[cfg(test)]
    pub(crate) fn build_frame_with_style_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_frame_with_style_into_with_motion(
            layout,
            style,
            model,
            frame,
            self.pulse_phase,
            true,
        );
    }

    /// Build a frame without animated values into reusable buffers.
    pub(crate) fn build_frame_with_style_into_static(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
    ) {
        self.build_frame_with_style_into_with_motion(layout, style, model, frame, 0.0, false);
    }

    /// Build one static segment bucket into reusable buffers.
    pub(crate) fn build_static_segment_with_style_into(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        motion_model: Option<&NativeMotionModel>,
        segment: StaticFrameSegment,
        segments: &mut StaticFrameSegments,
    ) {
        {
            let frame = segments.frame_mut(segment);
            frame.clear_color = style.clear_color;
            frame.primitives.clear();
            frame.text_runs.clear();
        }
        let emit_context = RefCell::new(SegmentedStaticEmitContext {
            layout,
            model,
            segments,
            target_segment: Some(segment),
        });
        let mut primitives = SegmentedPrimitiveSink {
            context: &emit_context,
        };
        let mut text_runs = SegmentedTextRunSink {
            context: &emit_context,
        };
        self.build_frame_with_style_into_with_motion_sinks(
            layout,
            style,
            model,
            &mut primitives,
            &mut text_runs,
            0.0,
            false,
            motion_model,
            Some(segment),
        );
    }

    /// Build a frame with a caller-supplied motion phase.
    fn build_frame_with_style_into_with_motion(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        model: &AppModel,
        frame: &mut NativeViewFrame,
        pulse_phase: f32,
        include_overlays: bool,
    ) {
        frame.clear_color = style.clear_color;
        frame.primitives.clear();
        frame.text_runs.clear();
        self.build_frame_with_style_into_with_motion_sinks(
            layout,
            style,
            model,
            &mut frame.primitives,
            &mut frame.text_runs,
            pulse_phase,
            include_overlays,
            None,
            None,
        );
    }

    /// Build a native frame using default style tokens.
    #[cfg(test)]
    pub(crate) fn build_frame(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> NativeViewFrame {
        self.build_frame_with_style(layout, &style_for_layout(layout), model)
    }
}
