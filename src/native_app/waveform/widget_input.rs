use radiant::{
    gui::types::{Rect, Vector2},
    widgets::{
        CanvasGestureEvent, CanvasPointer, DragHandleMessage, PointerButton, WidgetInput,
        WidgetOutput,
    },
};

use super::{
    WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionKind,
    WaveformWidget,
};

const SELECTION_CLICK_SLOP_PX: f32 = 2.0;

impl WaveformWidget {
    pub(in crate::native_app::waveform) fn handle_waveform_input(
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
        if let Some(pointer) = self.active_drag_motion_pointer(&event) {
            self.common.state.hovered = pointer_inside;
            self.clear_waveform_hover();
            if !has_loaded_sample {
                return None;
            }
            return self.active_drag_motion_output(&event, pointer);
        }
        if let Some(pointer) = event.hover_pointer() {
            self.common.state.hovered = pointer_inside;
            if !has_loaded_sample {
                self.clear_waveform_hover();
                return None;
            }
            if !pointer_inside {
                self.clear_waveform_hover();
                return None;
            }
            self.hovered_edit_fade_outer_gain_handle =
                self.edit_fade_outer_gain_handle_at(bounds, pointer.position);
            if self.hovered_edit_fade_outer_gain_handle.is_some() {
                self.hovered_edit_fade_handle = None;
                self.hovered_selection_handle = None;
                self.hovered_edit_gain_handle = false;
                self.hovered_similar_section = None;
                self.hover_cursor_ratio = None;
                return Some(pointer_location_output(pointer));
            }
            self.hovered_edit_fade_handle = self.edit_fade_handle_at(bounds, pointer.position);
            if self.hovered_edit_fade_handle.is_some() {
                self.hovered_edit_fade_outer_gain_handle = None;
                self.hovered_selection_handle = None;
                self.hovered_edit_gain_handle = false;
                self.hovered_similar_section = None;
                self.hover_cursor_ratio = None;
                return Some(pointer_location_output(pointer));
            }
            self.hovered_edit_gain_handle = self.edit_gain_handle_at(bounds, pointer.position);
            if self.hovered_edit_gain_handle {
                self.hovered_edit_fade_outer_gain_handle = None;
                self.hovered_selection_handle = None;
                self.hovered_similar_section = None;
                self.hover_cursor_ratio = None;
                return Some(pointer_location_output(pointer));
            }
            self.hovered_selection_handle =
                self.selection_handle_hover_at(bounds, pointer.position);
            if self.hovered_selection_handle.is_some() {
                self.hovered_similar_section = None;
                self.hover_cursor_ratio = None;
                return Some(pointer_location_output(pointer));
            }
            self.hovered_similar_section = self.similar_section_at(bounds, pointer.position);
            if self.hovered_similar_section.is_some() {
                self.hovered_edit_fade_outer_gain_handle = None;
                self.hovered_edit_fade_handle = None;
                self.hovered_edit_gain_handle = false;
                self.hover_cursor_ratio = None;
                return Some(pointer_location_output(pointer));
            }
            let visible_ratio = pointer.normalized_x();
            self.hover_cursor_ratio = self.absolute_ratio_for_visible(visible_ratio);
            return Some(pointer_location_output(pointer));
        }
        if let Some((pointer, delta)) = event.wheel_pointer_delta_inside(bounds) {
            let expand_silence_margin =
                matches!(&input, WidgetInput::Wheel { modifiers, .. } if modifiers.shift);
            return has_loaded_sample.then(|| {
                WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: pointer.normalized_x(),
                    expand_silence_margin,
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
            if self.active_drag_kind == Some(WaveformActiveDragKind::EditGain) {
                return Some(WidgetOutput::typed(WaveformInteraction::FinishEditGain {
                    pointer_y: pointer.position.y,
                }));
            }
            if matches!(
                self.active_drag_kind,
                Some(WaveformActiveDragKind::EditFadeOuterGain(_))
            ) {
                return Some(WidgetOutput::typed(
                    WaveformInteraction::FinishEditFadeOuterGain {
                        vertical_ratio: pointer.normalized_y(),
                    },
                ));
            }
            if self.primary_release_finishes_drag() {
                return Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                    visible_ratio: self.finish_selection_visible_ratio(&event, pointer),
                }));
            }
        }
        if let Some(pointer) = event.release_pointer(PointerButton::Secondary)
            && self.active_drag_kind == Some(WaveformActiveDragKind::EditGain)
        {
            return Some(WidgetOutput::typed(WaveformInteraction::FinishEditGain {
                pointer_y: pointer.position.y,
            }));
        }
        if let Some(pointer) = event.release_pointer(PointerButton::Secondary)
            && matches!(
                self.active_drag_kind,
                Some(WaveformActiveDragKind::EditFadeOuterGain(_))
            )
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::FinishEditFadeOuterGain {
                    vertical_ratio: pointer.normalized_y(),
                },
            ));
        }
        if let Some(pointer) = event.release_pointer(PointerButton::Secondary)
            && self.secondary_release_finishes_drag()
        {
            return Some(WidgetOutput::typed(WaveformInteraction::FinishSelection {
                visible_ratio: self.finish_selection_visible_ratio(&event, pointer),
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

    fn active_drag_motion_pointer(&self, event: &CanvasGestureEvent) -> Option<CanvasPointer> {
        self.active_drag_kind?;
        match event {
            CanvasGestureEvent::Drag { pointer, .. } => Some(*pointer),
            CanvasGestureEvent::Hover(pointer) => Some(*pointer),
            _ => None,
        }
    }

    fn clear_waveform_hover(&mut self) {
        self.hover_cursor_ratio = None;
        self.hovered_selection_handle = None;
        self.hovered_edit_fade_handle = None;
        self.hovered_edit_fade_outer_gain_handle = None;
        self.hovered_edit_gain_handle = false;
        self.hovered_similar_section = None;
    }

    fn active_drag_motion_output(
        &self,
        event: &CanvasGestureEvent,
        pointer: CanvasPointer,
    ) -> Option<WidgetOutput> {
        if self.active_drag_kind == Some(WaveformActiveDragKind::PlaySelectionExport) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::moved(
                    pointer.position,
                )),
            ));
        }
        if self.active_drag_kind == Some(WaveformActiveDragKind::EditGain) {
            return Some(WidgetOutput::typed(WaveformInteraction::UpdateEditGain {
                pointer_y: pointer.position.y,
            }));
        }
        if matches!(
            self.active_drag_kind,
            Some(WaveformActiveDragKind::EditFadeOuterGain(_))
        ) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::UpdateEditFadeOuterGain {
                    vertical_ratio: pointer.normalized_y(),
                },
            ));
        }
        if self.selection_drag_is_inside_click_slop(event) {
            return None;
        }
        Some(WidgetOutput::typed(WaveformInteraction::UpdateSelection {
            visible_ratio: pointer.normalized_x(),
        }))
    }

    fn handle_primary_press(
        &mut self,
        bounds: Rect,
        pointer: CanvasPointer,
    ) -> Option<WidgetOutput> {
        let position = pointer.position;
        let visible_ratio = pointer.normalized_x();
        self.clear_waveform_hover();
        if self.play_selection_export_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::DragPlaySelectionExport(DragHandleMessage::started(position)),
            ));
        }
        if let Some(handle) = self.edit_fade_outer_gain_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginEditFadeOuterGain {
                    handle,
                    vertical_ratio: pointer.normalized_y(),
                },
            ));
        }
        if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio,
            }));
        }
        if self.edit_gain_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditGain {
                pointer_y: position.y,
            }));
        }
        if let Some(edge) =
            self.selection_resize_handle_at(bounds, position, WaveformSelectionKind::Edit)
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionResize {
                    kind: WaveformSelectionKind::Edit,
                    edge,
                    visible_ratio,
                },
            ));
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
        if let Some(selection) = self.similar_section_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::SelectSimilarSection { selection },
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

    fn handle_secondary_press(
        &mut self,
        bounds: Rect,
        pointer: CanvasPointer,
    ) -> Option<WidgetOutput> {
        let position = pointer.position;
        let visible_ratio = pointer.normalized_x();
        self.clear_waveform_hover();
        if let Some(handle) = self.edit_fade_outer_gain_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginEditFadeOuterGain {
                    handle,
                    vertical_ratio: pointer.normalized_y(),
                },
            ));
        }
        if let Some(handle) = self.edit_fade_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio,
            }));
        }
        if self.edit_gain_handle_at(bounds, position) {
            return Some(WidgetOutput::typed(WaveformInteraction::BeginEditGain {
                pointer_y: position.y,
            }));
        }
        if let Some(edge) =
            self.selection_resize_handle_at(bounds, position, WaveformSelectionKind::Edit)
        {
            return Some(WidgetOutput::typed(
                WaveformInteraction::BeginSelectionResize {
                    kind: WaveformSelectionKind::Edit,
                    edge,
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
        if let Some(selection) = self.similar_section_at(bounds, position) {
            return Some(WidgetOutput::typed(
                WaveformInteraction::SelectSimilarSection { selection },
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
                        | WaveformActiveDragKind::SelectionResize(_, _)
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
                        | WaveformActiveDragKind::SelectionResize(WaveformSelectionKind::Edit, _)
                        | WaveformActiveDragKind::SelectionMove(WaveformSelectionKind::Edit)
                )
            )
    }

    fn selection_drag_is_inside_click_slop(&self, event: &CanvasGestureEvent) -> bool {
        if !self.active_drag_is_selection_like() {
            return false;
        }
        matches!(
            event,
            CanvasGestureEvent::Drag { delta, .. }
                if horizontal_delta_inside_click_slop(*delta)
        )
    }

    fn finish_selection_visible_ratio(
        &self,
        event: &CanvasGestureEvent,
        fallback: CanvasPointer,
    ) -> f32 {
        if !self.active_drag_is_selection_like() {
            return fallback.normalized_x();
        }
        match event {
            CanvasGestureEvent::Release { origin, delta, .. }
                if horizontal_delta_inside_click_slop(*delta) =>
            {
                origin.normalized_x()
            }
            _ => fallback.normalized_x(),
        }
    }

    fn active_drag_is_selection_like(&self) -> bool {
        matches!(
            self.active_drag_kind,
            Some(
                WaveformActiveDragKind::Selection(_)
                    | WaveformActiveDragKind::SelectionResize(_, _)
                    | WaveformActiveDragKind::SelectionMove(_)
            )
        )
    }
}

fn horizontal_delta_inside_click_slop(delta: Vector2) -> bool {
    delta.x.abs() <= SELECTION_CLICK_SLOP_PX
}

fn pointer_location_output(pointer: CanvasPointer) -> WidgetOutput {
    WidgetOutput::typed(WaveformInteraction::RememberPointerLocation {
        position: pointer.position,
    })
}
