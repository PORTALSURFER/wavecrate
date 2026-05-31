use radiant::{
    gui::types::Rect,
    widgets::{
        CanvasGestureEvent, CanvasPointer, DragHandleMessage, PointerButton, WidgetInput,
        WidgetOutput,
    },
};

use super::{
    WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionKind,
    WaveformWidget,
};

impl WaveformWidget {
    pub(in crate::gui_app::waveform) fn handle_waveform_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        if input_starts_outside(bounds, &input) {
            return None;
        }
        let event = self.gesture.handle_input(bounds, &input)?;
        match event {
            CanvasGestureEvent::Hover(pointer) => {
                self.common.state.hovered = bounds.contains(pointer.position);
                if !self.has_loaded_sample() {
                    return None;
                }
                if self.active_drag_kind == Some(WaveformActiveDragKind::PlaySelectionExport) {
                    return Some(WidgetOutput::typed(
                        WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::Moved {
                            position: pointer.position,
                        }),
                    ));
                }
                self.active_drag_kind.map(|_| {
                    WidgetOutput::typed(WaveformInteraction::UpdateSelection {
                        visible_ratio: visible_ratio(pointer),
                    })
                })
            }
            CanvasGestureEvent::Wheel { pointer, .. } if !bounds.contains(pointer.position) => None,
            CanvasGestureEvent::Wheel { pointer, .. } if !self.has_loaded_sample() => None,
            CanvasGestureEvent::Wheel { pointer, delta } => {
                Some(WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: visible_ratio(pointer),
                }))
            }
            CanvasGestureEvent::Press {
                pointer,
                button: PointerButton::Primary,
                ..
            } if self.has_loaded_sample() && bounds.contains(pointer.position) => {
                self.handle_primary_press(bounds, pointer)
            }
            CanvasGestureEvent::DoubleClick {
                pointer,
                button: PointerButton::Primary,
                ..
            } if self.has_loaded_sample() && bounds.contains(pointer.position) => {
                self.handle_primary_double_click(bounds, pointer)
            }
            CanvasGestureEvent::Press {
                pointer,
                button: PointerButton::Secondary,
                ..
            } if self.has_loaded_sample() && bounds.contains(pointer.position) => {
                self.handle_secondary_press(bounds, pointer)
            }
            CanvasGestureEvent::Press {
                pointer,
                button: PointerButton::Auxiliary,
                ..
            } if self.has_loaded_sample() && bounds.contains(pointer.position) => {
                Some(WidgetOutput::typed(WaveformInteraction::BeginPan {
                    visible_ratio: visible_ratio(pointer),
                }))
            }
            CanvasGestureEvent::Release {
                pointer,
                button: PointerButton::Primary,
                ..
            } if self.has_loaded_sample()
                && self.active_drag_kind == Some(WaveformActiveDragKind::PlaySelectionExport) =>
            {
                Some(WidgetOutput::typed(
                    WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::Ended {
                        position: pointer.position,
                    }),
                ))
            }
            CanvasGestureEvent::Release {
                pointer,
                button: PointerButton::Primary,
                ..
            } if self.has_loaded_sample() && self.primary_release_finishes_drag() => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: visible_ratio(pointer),
                }))
            }
            CanvasGestureEvent::Release {
                pointer,
                button: PointerButton::Secondary,
                ..
            } if self.has_loaded_sample() && self.secondary_release_finishes_drag() => {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: visible_ratio(pointer),
                }))
            }
            CanvasGestureEvent::Release {
                pointer,
                button: PointerButton::Auxiliary,
                ..
            } if self.has_loaded_sample()
                && self.active_drag_kind == Some(WaveformActiveDragKind::Pan) =>
            {
                Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: visible_ratio(pointer),
                }))
            }
            _ => None,
        }
    }

    fn handle_primary_press(&self, bounds: Rect, pointer: CanvasPointer) -> Option<WidgetOutput> {
        let position = pointer.position;
        let visible_ratio = visible_ratio(pointer);
        if self.play_selection_export_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::Started {
                    position,
                }),
            ));
        }
        if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio,
            }));
        }
        if let Some(edge) =
            self.selection_resize_handle_at(bounds, position, WaveformSelectionKind::Play)
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionResize {
                    kind: WaveformSelectionKind::Play,
                    edge,
                    visible_ratio,
                },
            ));
        }
        if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Play) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionMove {
                    kind: WaveformSelectionKind::Play,
                    visible_ratio,
                },
            ));
        }
        if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Edit) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionMove {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio,
                },
            ));
        }
        Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio,
        }))
    }

    fn handle_primary_double_click(
        &self,
        bounds: Rect,
        pointer: CanvasPointer,
    ) -> Option<WidgetOutput> {
        let position = pointer.position;
        if let Some(
            handle @ (WaveformEditFadeHandle::InOuterStart | WaveformEditFadeHandle::OutOuterEnd),
        ) = self.edit_fade_handle_at(bounds, position)
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::ClearEditFadeSilence { handle },
            ));
        }
        None
    }

    fn handle_secondary_press(&self, bounds: Rect, pointer: CanvasPointer) -> Option<WidgetOutput> {
        let position = pointer.position;
        let visible_ratio = visible_ratio(pointer);
        if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio,
            }));
        }
        if self.selection_move_handle_at(bounds, position, WaveformSelectionKind::Edit) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionMove {
                    kind: WaveformSelectionKind::Edit,
                    visible_ratio,
                },
            ));
        }
        Some(WidgetOutput::typed(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Edit,
            visible_ratio,
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

fn visible_ratio(pointer: CanvasPointer) -> f32 {
    pointer.normalized.x
}

fn input_starts_outside(bounds: Rect, input: &WidgetInput) -> bool {
    match input {
        WidgetInput::PointerPress { position, .. }
        | WidgetInput::PointerDoubleClick { position, .. }
        | WidgetInput::Wheel { position, .. } => !bounds.contains(*position),
        _ => false,
    }
}
