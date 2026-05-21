use radiant::{
    gui::types::{Point, Rect, Vector2},
    gui::visualization::TimelineEditPreview,
    layout::LayoutOutput,
    prelude as ui,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, PointerButton, Widget, WidgetCommon, WidgetInput, WidgetOutput,
        WidgetSizing,
    },
};
use std::sync::Arc;

use crate::gui_app::GuiMessage;

use super::{
    WAVEFORM_HEIGHT, WAVEFORM_WIDTH, WaveformActiveDragKind, WaveformEditFadeHandle, WaveformFile,
    WaveformInteraction, WaveformSelectionKind, WaveformSignalWidget, WaveformState,
    WaveformViewport, edit_preview_for_selection,
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
        .id(12)
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
    play_selection_flash_frames: u8,
    active_drag_kind: Option<WaveformActiveDragKind>,
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
    pub(super) play_selection_flash_frames: u8,
    pub(super) edit_preview: TimelineEditPreview,
    active_drag_kind: Option<WaveformActiveDragKind>,
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
            play_selection_flash_frames,
            edit_preview: edit_preview_for_selection(edit_selection),
            active_drag_kind,
        }
    }

    fn ratio_from_position(&self, bounds: Rect, position: Point) -> f32 {
        ((position.x - bounds.min.x) / bounds.width().max(1.0)).clamp(0.0, 1.0)
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
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.active_drag_kind.map(|_| {
                    WidgetOutput::typed(WaveformInteraction::UpdateSelection {
                        visible_ratio: self.ratio_from_position(bounds, position),
                    })
                })
            }
            WidgetInput::Wheel { position, delta } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => self.handle_primary_press(bounds, position),
            WidgetInput::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => self.handle_primary_double_click(bounds, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Secondary,
                ..
            } if bounds.contains(position) => self.handle_secondary_press(bounds, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Auxiliary,
                ..
            } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::BeginPan {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } if self.primary_release_finishes_drag() => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Secondary,
                ..
            } if self.secondary_release_finishes_drag() => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Auxiliary,
                ..
            } if self.active_drag_kind == Some(WaveformActiveDragKind::Pan) => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            _ => None,
        }
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

impl WaveformWidget {
    fn handle_primary_press(&self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio: self.ratio_from_position(bounds, position),
            }));
        }
        if let Some(edge) =
            self.selection_resize_handle_at(bounds, position, WaveformSelectionKind::Play)
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionResize {
                    kind: WaveformSelectionKind::Play,
                    edge,
                    visible_ratio: self.ratio_from_position(bounds, position),
                },
            ));
        }
        if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Play) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionMove {
                    kind: WaveformSelectionKind::Play,
                    visible_ratio: self.ratio_from_position(bounds, position),
                },
            ));
        }
        if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Edit) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionMove {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio: self.ratio_from_position(bounds, position),
                },
            ));
        }
        Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: self.ratio_from_position(bounds, position),
        }))
    }

    fn handle_primary_double_click(&self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        if let Some(
            handle @ (WaveformEditFadeHandle::FadeInOuterStart
            | WaveformEditFadeHandle::FadeOutOuterEnd),
        ) = self.edit_fade_handle_at(bounds, position)
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::ClearEditFadeSilence { handle },
            ));
        }
        None
    }

    fn handle_secondary_press(&self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio: self.ratio_from_position(bounds, position),
            }));
        }
        if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Edit) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionMove {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio: self.ratio_from_position(bounds, position),
                },
            ));
        }
        Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio: self.ratio_from_position(bounds, position),
        }))
    }

    fn primary_release_finishes_drag(&self) -> bool {
        self.active_drag_kind
            == Some(WaveformActiveDragKind::Selection(
                WaveformSelectionKind::Play,
            ))
            || matches!(
                self.active_drag_kind,
                Some(
                    WaveformActiveDragKind::EditFade(_)
                        | WaveformActiveDragKind::SelectionResize(WaveformSelectionKind::Play, _)
                        | WaveformActiveDragKind::SelectionMove(_)
                )
            )
    }

    fn secondary_release_finishes_drag(&self) -> bool {
        self.active_drag_kind
            == Some(WaveformActiveDragKind::Selection(
                WaveformSelectionKind::Edit,
            ))
            || matches!(
                self.active_drag_kind,
                Some(
                    WaveformActiveDragKind::EditFade(_)
                        | WaveformActiveDragKind::SelectionMove(WaveformSelectionKind::Edit)
                )
            )
    }
}
