use radiant::{
    gui::types::{Rect, Vector2},
    gui::visualization::TimelineEditPreview,
    layout::LayoutOutput,
    prelude as ui,
    runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, PaintPrimitive},
    theme::ThemeTokens,
    widgets::{CanvasGestureState, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing},
};
use std::sync::Arc;

use crate::native_app::app::GuiMessage;
use crate::native_app::ui::ids as widget_ids;
use crate::native_app::waveform::{WAVEFORM_SIGNAL_WIDGET_ID, WAVEFORM_WIDGET_ID};

use super::{
    WAVEFORM_HEIGHT, WAVEFORM_WIDTH, WaveformActiveDragKind, WaveformEditFadeHandle,
    WaveformEditFadeOuterGainHandle, WaveformFile, WaveformInteraction, WaveformState,
    WaveformViewport, audio_file::gain_preview_for_selection, edit_preview_for_selection,
    widget_geometry::WaveformSelectionHandleHover,
};

pub(in crate::native_app) fn waveform_viewport_view_with_tooltip(
    state: &WaveformState,
    tooltip: Option<&'static str>,
    beat_guides_enabled: bool,
    beat_guide_count: u8,
) -> ui::View<GuiMessage> {
    let interaction = ui::custom_widget(
        WaveformWidget::new(WaveformWidgetProps::from_state(
            state,
            beat_guides_enabled,
            beat_guide_count,
        )),
        |output| {
            output
                .typed_copied::<WaveformInteraction>()
                .map(GuiMessage::Waveform)
        },
    )
    .id(WAVEFORM_WIDGET_ID)
    .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32);
    let interaction = if let Some(tooltip) = tooltip {
        interaction.tooltip(tooltip)
    } else {
        interaction
    };

    ui::stack([
        waveform_signal_surface_view(state.file(), state.viewport(), state.edit_selection())
            .id(WAVEFORM_SIGNAL_WIDGET_ID)
            .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
        interaction,
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
            runtime_overlays: Default::default(),
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
    hover_cursor_ratio: Option<f32>,
    hovered_selection_handle: Option<WaveformSelectionHandleHover>,
    hovered_edit_fade_handle: Option<WaveformEditFadeHandle>,
    hovered_edit_fade_outer_gain_handle: Option<WaveformEditFadeOuterGainHandle>,
    hovered_edit_gain_handle: bool,
    hovered_similar_section: Option<wavecrate::selection::SelectionRange>,
    extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    similar_section_ranges: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    edit_selection_flash_frames: u8,
    beat_guides_enabled: bool,
    beat_guide_count: u8,
    playing: bool,
    pub(in crate::native_app::waveform) active_drag_kind: Option<WaveformActiveDragKind>,
}

impl WaveformWidgetProps {
    pub(super) fn from_state(
        state: &WaveformState,
        beat_guides_enabled: bool,
        beat_guide_count: u8,
    ) -> Self {
        Self {
            file: state.file(),
            viewport: state.viewport(),
            playhead_ratio: state.playhead_ratio(),
            play_mark_ratio: state.play_mark_ratio(),
            edit_mark_ratio: state.edit_mark_ratio(),
            play_selection: state.play_selection(),
            edit_selection: state.edit_selection(),
            hover_cursor_ratio: None,
            hovered_selection_handle: None,
            hovered_edit_fade_handle: None,
            hovered_edit_fade_outer_gain_handle: None,
            hovered_edit_gain_handle: false,
            hovered_similar_section: None,
            extracted_ranges: state.extracted_ranges().to_vec(),
            similar_section_ranges: state.similar_section_ranges().to_vec(),
            play_selection_flash_frames: state.play_selection_flash_frames(),
            edit_selection_flash_frames: state.edit_selection_flash_frames(),
            beat_guides_enabled,
            beat_guide_count,
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
    pub(super) hover_cursor_ratio: Option<f32>,
    pub(super) hovered_selection_handle: Option<WaveformSelectionHandleHover>,
    pub(super) hovered_edit_fade_handle: Option<WaveformEditFadeHandle>,
    pub(super) hovered_edit_fade_outer_gain_handle: Option<WaveformEditFadeOuterGainHandle>,
    pub(super) hovered_edit_gain_handle: bool,
    pub(super) hovered_similar_section: Option<wavecrate::selection::SelectionRange>,
    pub(super) extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    pub(super) similar_section_ranges: Vec<wavecrate::selection::SelectionRange>,
    pub(super) play_selection_flash_frames: u8,
    pub(super) edit_selection_flash_frames: u8,
    pub(super) beat_guides_enabled: bool,
    pub(super) beat_guide_count: u8,
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
            hover_cursor_ratio,
            hovered_selection_handle,
            hovered_edit_fade_handle,
            hovered_edit_fade_outer_gain_handle,
            hovered_edit_gain_handle,
            hovered_similar_section,
            extracted_ranges,
            similar_section_ranges,
            play_selection_flash_frames,
            edit_selection_flash_frames,
            beat_guides_enabled,
            beat_guide_count,
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
            hover_cursor_ratio,
            hovered_selection_handle,
            hovered_edit_fade_handle,
            hovered_edit_fade_outer_gain_handle,
            hovered_edit_gain_handle,
            hovered_similar_section,
            extracted_ranges,
            similar_section_ranges,
            play_selection_flash_frames,
            edit_selection_flash_frames,
            beat_guides_enabled,
            beat_guide_count,
            playing,
            edit_preview: edit_preview_for_selection(edit_selection),
            active_drag_kind,
        }
    }

    pub(super) fn has_loaded_sample(&self) -> bool {
        self.file.has_loaded_sample_metadata()
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
        true
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
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

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        self.append_hover_edit_fade_handle_paint(primitives, bounds);
        self.append_hover_edit_fade_outer_gain_handle_paint(primitives, bounds);
        self.append_hover_selection_handle_paint(primitives, bounds);
        self.append_hover_similar_section_paint(primitives, bounds);
        self.append_hover_cursor_paint(primitives, bounds);
    }
}
