use radiant::{
    gui::types::Rect,
    widgets::{CanvasPointer, DragHandleMessage, PointerButton, WidgetInput, WidgetOutput},
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
        if input.pointer_start_outside(bounds) {
            return None;
        }
        let event = self.gesture.handle_input(bounds, &input)?;
        let pointer_inside = event.pointer_is_inside(bounds);
        let has_loaded_sample = self.has_loaded_sample();
        if let Some(pointer) = event.hover_pointer() {
            self.common.state.hovered = pointer_inside;
            if !has_loaded_sample {
                return None;
            }
            if self.active_drag_kind == Some(WaveformActiveDragKind::PlaySelectionExport) {
                return Some(WidgetOutput::typed(
                    WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::moved(
                        pointer.position,
                    )),
                ));
            }
            return self.active_drag_kind.map(|_| {
                WidgetOutput::typed(WaveformInteraction::UpdateSelection {
                    visible_ratio: pointer.normalized_x(),
                })
            });
        }
        if let Some((pointer, delta)) = event.wheel_pointer_delta_inside(bounds) {
            return has_loaded_sample.then(|| {
                WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: pointer.normalized_x(),
                })
            });
        }
        if !has_loaded_sample {
            return None;
        }
        if let Some(pointer) = event.press_pointer_inside(bounds, PointerButton::Primary) {
            return self.handle_primary_press(bounds, pointer);
        }
        if let Some(pointer) = event.double_click_pointer_inside(bounds, PointerButton::Primary) {
            return self.handle_primary_double_click(bounds, pointer);
        }
        if let Some(pointer) = event.press_pointer_inside(bounds, PointerButton::Secondary) {
            return self.handle_secondary_press(bounds, pointer);
        }
        if let Some(pointer) = event.press_pointer_inside(bounds, PointerButton::Auxiliary) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginPan {
                visible_ratio: pointer.normalized_x(),
            }));
        }
        if let Some(pointer) = event.release_pointer(PointerButton::Primary) {
            if self.active_drag_kind == Some(WaveformActiveDragKind::PlaySelectionExport) {
                return Some(WidgetOutput::typed(
                    WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::ended(
                        pointer.position,
                    )),
                ));
            }
            if self.primary_release_finishes_drag() {
                return Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: pointer.normalized_x(),
                }));
            }
        }
        if let Some(pointer) = event.release_pointer(PointerButton::Secondary)
            && self.secondary_release_finishes_drag()
        {
            return Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                visible_ratio: pointer.normalized_x(),
            }));
        }
        if let Some(pointer) = event.release_pointer(PointerButton::Auxiliary)
            && self.active_drag_kind == Some(WaveformActiveDragKind::Pan)
        {
            return Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                visible_ratio: pointer.normalized_x(),
            }));
        }
        None
    }

    fn handle_primary_press(&self, bounds: Rect, pointer: CanvasPointer) -> Option<WidgetOutput> {
        let position = pointer.position;
        let visible_ratio = pointer.normalized_x();
        if self.play_selection_export_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::started(position)),
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
        let visible_ratio = pointer.normalized_x();
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
