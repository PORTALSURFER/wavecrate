use radiant::{
    gui::types::{Rect, Rgba8, Vector2},
    layout::LayoutOutput,
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceRuntimeOverlays,
        PaintGpuSurface, PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};
use std::sync::Arc;

use super::{
    BAND_COUNT, WAVEFORM_HEIGHT, WAVEFORM_WIDTH, WaveformActiveDragKind, WaveformFile,
    WaveformViewport, audio_file::gain_preview_for_selection,
};

#[derive(Clone, Debug)]
pub(super) struct WaveformSignalWidget {
    common: WidgetCommon,
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
}

impl WaveformSignalWidget {
    pub(super) fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        edit_selection: Option<wavecrate::selection::SelectionRange>,
        _active_drag_kind: Option<WaveformActiveDragKind>,
    ) -> Self {
        let common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)),
        )
        .without_default_chrome();
        Self {
            common,
            file,
            viewport,
            edit_selection,
        }
    }

    fn signal_summary(&self) -> Arc<radiant::runtime::GpuSignalSummary> {
        Arc::clone(&self.file.gpu_signal_summary)
    }

    fn signal_revision(&self) -> u64 {
        self.file.content_revision()
    }
}

impl Widget for WaveformSignalWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: self.file.path_hash(),
            revision: self.signal_revision(),
            rect: bounds,
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: self.file.frames,
                band_count: BAND_COUNT,
                frame_range: [self.viewport.start as f32, self.viewport.end as f32],
                summary: self.signal_summary(),
                gain_preview: gain_preview_for_selection(self.edit_selection),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                    GpuSurfaceLineStyle {
                        color: Rgba8 {
                            r: 255,
                            g: 255,
                            b: 255,
                            a: 235,
                        },
                        width: 1.0,
                    },
                ),
            },
            overlays: Vec::new(),
        }));
    }
}
