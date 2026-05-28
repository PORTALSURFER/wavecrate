use radiant::{
    gui::types::{Rect, Vector2},
    gui::visualization::TimelineEditPreview,
    layout::LayoutOutput,
    prelude as ui,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;

use crate::gui_app::{GuiMessage, WAVEFORM_WIDGET_ID};

use super::{
    WAVEFORM_HEIGHT, WAVEFORM_WIDTH, WaveformActiveDragKind, WaveformFile, WaveformInteraction,
    WaveformSignalWidget, WaveformState, WaveformViewport, edit_preview_for_selection,
};

pub(in crate::gui_app) fn waveform_viewport_view(state: &WaveformState) -> ui::View<GuiMessage> {
    ui::stack([
        ui::custom_widget(
            WaveformSignalWidget::new(
                state.file(),
                state.viewport(),
                state.edit_selection(),
                state.active_drag_kind(),
            ),
            |_| None,
        )
        .id(11)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
        ui::custom_widget(
            WaveformWidget::new(WaveformWidgetProps::from_state(state)),
            |output| {
                output
                    .typed_ref::<WaveformInteraction>()
                    .copied()
                    .map(GuiMessage::Waveform)
            },
        )
        .id(WAVEFORM_WIDGET_ID)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32),
    ])
    .id(10)
    .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct WaveformWidgetProps {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    pub(in crate::gui_app::waveform) active_drag_kind: Option<WaveformActiveDragKind>,
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
            active_drag_kind: state.active_drag_kind(),
        }
    }
}

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct WaveformWidget {
    pub(super) common: WidgetCommon,
    pub(super) file: Arc<WaveformFile>,
    pub(super) viewport: WaveformViewport,
    pub(super) playhead_ratio: Option<f32>,
    pub(super) play_mark_ratio: Option<f32>,
    pub(super) edit_mark_ratio: Option<f32>,
    pub(super) play_selection: Option<wavecrate::selection::SelectionRange>,
    pub(super) edit_selection: Option<wavecrate::selection::SelectionRange>,
    pub(super) extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    pub(super) play_selection_flash_frames: u8,
    pub(super) edit_preview: TimelineEditPreview,
    pub(in crate::gui_app::waveform) active_drag_kind: Option<WaveformActiveDragKind>,
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
            active_drag_kind,
        } = props;
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            file,
            viewport,
            playhead_ratio,
            play_mark_ratio,
            edit_mark_ratio,
            play_selection,
            edit_selection,
            extracted_ranges,
            play_selection_flash_frames,
            edit_preview: edit_preview_for_selection(edit_selection),
            active_drag_kind,
        }
    }

    pub(super) fn has_loaded_sample(&self) -> bool {
        !self.file.audio_bytes.is_empty() && !self.file.path.as_os_str().is_empty()
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
