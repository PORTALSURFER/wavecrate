use radiant::{
    gui::types::{Rect, Vector2},
    gui::visualization::TimelineEditPreview,
    layout::LayoutOutput,
    prelude as ui,
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceLineStyle, GpuSurfaceRuntimeOverlays,
        PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{CanvasGestureState, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};
use std::sync::Arc;

use crate::native_app::app::GuiMessage;
use crate::native_app::ui::ids as widget_ids;
use crate::native_app::waveform::{WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID};

use super::{
    WAVEFORM_HEIGHT, WAVEFORM_WIDTH, WaveformActiveDragKind, WaveformFile, WaveformInteraction,
    WaveformState, WaveformViewport, audio_file::gain_preview_for_selection,
    edit_preview_for_selection,
};

pub(in crate::native_app) fn waveform_viewport_view(state: &WaveformState) -> ui::View<GuiMessage> {
    ui::stack([
        waveform_signal_surface_view(state.file(), state.viewport(), state.edit_selection())
            .id(WAVEFORM_SIGNAL_WIDGET_ID)
            .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
        ui::custom_widget(
            WaveformWidget::new(WaveformWidgetProps::from_state(state)),
            |output| {
                output
                    .typed_copied::<WaveformInteraction>()
                    .map(GuiMessage::Waveform)
            },
        )
        .id(WAVEFORM_WIDGET_ID)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
    ])
    .id(widget_ids::WAVEFORM_VIEWPORT_STACK_ID)
    .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
}

pub(in crate::native_app::waveform) fn waveform_signal_surface_view(
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
) -> ui::View<GuiMessage> {
    ui::gpu_surface_configured_from_parts(
        ui::GpuSurfaceConfiguredParts::new(
            file.path_hash(),
            file.content_revision(),
            GpuSurfaceContent::SignalSummaryBands {
                frames: file.frames,
                band_count: super::BAND_COUNT,
                frame_range: [viewport.start as f32, viewport.end as f32],
                summary: Arc::clone(&file.gpu_signal_summary),
                gain_preview: gain_preview_for_selection(edit_selection),
            },
        )
        .capabilities(GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: GpuSurfaceRuntimeOverlays::pointer_vertical_line(
                GpuSurfaceLineStyle {
                    color: ui::Rgba8 {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 235,
                    },
                    width: 1.0,
                },
            ),
        }),
    )
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformWidgetProps {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    playing: bool,
    pub(in crate::native_app::waveform) active_drag_kind: Option<WaveformActiveDragKind>,
}

impl WaveformWidgetProps {
    pub(super) fn from_state(state: &WaveformState) -> Self {
        Self {
            file: state.file(),
            viewport: state.viewport(),
            playhead_ratio: state.playhead_ratio(),
            play_mark_ratio: state.play_mark_ratio(),
            edit_mark_ratio: state.edit_mark_ratio(),
            play_selection: state.play_selection(),
            edit_selection: state.edit_selection(),
            extracted_ranges: state.extracted_ranges().to_vec(),
            play_selection_flash_frames: state.play_selection_flash_frames(),
            playing: state.is_playing(),
            active_drag_kind: state.active_drag_kind(),
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformWidget {
    pub(super) common: WidgetCommon,
    pub(super) gesture: CanvasGestureState,
    pub(super) file: Arc<WaveformFile>,
    pub(super) viewport: WaveformViewport,
    pub(super) playhead_ratio: Option<f32>,
    pub(super) play_mark_ratio: Option<f32>,
    pub(super) edit_mark_ratio: Option<f32>,
    pub(super) play_selection: Option<wavecrate::selection::SelectionRange>,
    pub(super) edit_selection: Option<wavecrate::selection::SelectionRange>,
    pub(super) extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    pub(super) play_selection_flash_frames: u8,
    pub(super) playing: bool,
    pub(super) edit_preview: TimelineEditPreview,
    pub(in crate::native_app::waveform) active_drag_kind: Option<WaveformActiveDragKind>,
}

impl WaveformWidget {
    pub(super) fn new(props: WaveformWidgetProps) -> Self {
        let WaveformWidgetProps {
            file,
            viewport,
            playhead_ratio,
            play_mark_ratio,
            edit_mark_ratio,
            play_selection,
            edit_selection,
            extracted_ranges,
            play_selection_flash_frames,
            playing,
            active_drag_kind,
        } = props;
        let common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)),
        )
        .with_pointer_focus()
        .without_default_chrome();
        Self {
            common,
            gesture: CanvasGestureState::new(),
            file,
            viewport,
            playhead_ratio,
            play_mark_ratio,
            edit_mark_ratio,
            play_selection,
            edit_selection,
            extracted_ranges,
            play_selection_flash_frames,
            playing,
            edit_preview: edit_preview_for_selection(edit_selection),
            active_drag_kind,
        }
    }

    pub(super) fn has_loaded_sample(&self) -> bool {
        !self.file.path.as_os_str().is_empty()
            && (!self.file.audio_bytes.is_empty()
                || self.file.playback_samples.is_some()
                || self.file.playback_cache_file.is_some())
    }
}

impl Widget for WaveformWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_waveform_input(bounds, input)
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.append_selection_and_marker_paint(primitives, bounds);
        self.append_edit_fade_paint(primitives, bounds);
    }
}
