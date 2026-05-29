use super::{
    MIN_VISIBLE_FRAMES, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionKind,
    WaveformState,
    interaction::{
        WaveformDrag, WaveformEditFadeDrag, WaveformPanDrag, WaveformSelectionDrag,
        WaveformSelectionMoveDrag, WaveformSelectionResizeDrag,
    },
};

impl WaveformState {
    pub(in crate::gui_app) fn apply_interaction(&mut self, interaction: WaveformInteraction) {
        if !self.has_loaded_sample() && !matches!(interaction, WaveformInteraction::Frame) {
            self.active_drag = None;
            return;
        }
        match interaction {
            WaveformInteraction::Wheel {
                delta,
                anchor_ratio,
            } => {
                self.zoom_anchor_ratio = anchor_ratio;
                self.handle_wheel(delta, anchor_ratio);
            }
            WaveformInteraction::ScrollTo { offset_fraction } => {
                self.set_offset_fraction(offset_fraction);
            }
            WaveformInteraction::BeginSelection {
                kind,
                visible_ratio,
            } => {
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::Selection(WaveformSelectionDrag::new(
                    kind, ratio,
                )));
                match kind {
                    WaveformSelectionKind::Play => {
                        self.play_mark_ratio = Some(ratio);
                        self.play_selection = None;
                        self.play_selection_flash_frames = 0;
                    }
                    WaveformSelectionKind::Edit => {
                        self.edit_mark_ratio = Some(ratio);
                        self.edit_selection = None;
                    }
                }
            }
            WaveformInteraction::BeginEditFade {
                handle,
                visible_ratio,
            } => {
                let Some(selection) = self.edit_selection else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::EditFade(WaveformEditFadeDrag::new(
                    handle, selection,
                )));
                self.update_active_edit_fade(ratio);
            }
            WaveformInteraction::ClearEditFadeSilence { handle } => {
                self.clear_edit_fade_silence(handle);
                self.active_drag = None;
            }
            WaveformInteraction::BeginSelectionResize {
                kind,
                edge,
                visible_ratio,
            } => {
                let Some(selection) = self.selection_for_kind(kind) else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::SelectionResize(
                    WaveformSelectionResizeDrag::new(kind, edge, selection),
                ));
                self.update_active_selection_resize(ratio);
            }
            WaveformInteraction::BeginSelectionMove {
                kind,
                visible_ratio,
            } => {
                let Some(selection) = self.selection_for_kind(kind) else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                self.active_drag = Some(WaveformDrag::SelectionMove(
                    WaveformSelectionMoveDrag::new(kind, ratio, selection),
                ));
                self.update_active_selection_move(ratio);
            }
            WaveformInteraction::BeginPan { visible_ratio } => {
                self.active_drag = Some(WaveformDrag::Pan(WaveformPanDrag::new(
                    visible_ratio,
                    self.viewport
                        .clamp(self.file.frames.max(1), MIN_VISIBLE_FRAMES),
                )));
            }
            WaveformInteraction::UpdateSelection { visible_ratio } => {
                self.update_active_drag(visible_ratio);
            }
            WaveformInteraction::FinishSelection { visible_ratio } => {
                self.finish_active_drag(visible_ratio);
            }
            WaveformInteraction::DragPlaySelectionExport(drag) => {
                self.apply_play_selection_export_drag(drag);
            }
            WaveformInteraction::Frame => {
                self.play_selection_flash_frames =
                    self.play_selection_flash_frames.saturating_sub(1);
            }
        }
    }

    fn update_active_drag(&mut self, visible_ratio: f32) {
        let ratio = self.absolute_ratio_from_visible(visible_ratio);
        let Some(drag) = self.active_drag else {
            return;
        };
        match drag {
            WaveformDrag::Selection(mut drag) => {
                drag.update(ratio);
                self.active_drag = Some(WaveformDrag::Selection(drag));
                if drag.moved {
                    self.set_selection_for_drag(drag);
                }
            }
            WaveformDrag::EditFade(_) => {
                self.update_active_edit_fade(ratio);
            }
            WaveformDrag::SelectionResize(_) => {
                self.update_active_selection_resize(ratio);
            }
            WaveformDrag::SelectionMove(_) => {
                self.update_active_selection_move(ratio);
            }
            WaveformDrag::PlaySelectionExport => {}
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn finish_active_drag(&mut self, visible_ratio: f32) {
        let ratio = self.absolute_ratio_from_visible(visible_ratio);
        let Some(drag) = self.active_drag.take() else {
            return;
        };
        match drag {
            WaveformDrag::Selection(mut drag) => {
                drag.update(ratio);
                if drag.moved {
                    self.set_selection_for_drag(drag);
                    return;
                }
                match drag.kind {
                    WaveformSelectionKind::Play => {
                        self.play_selection = None;
                        self.start_playback(ratio);
                        self.pending_playback_start = Some(ratio);
                    }
                    WaveformSelectionKind::Edit => {
                        self.edit_selection = None;
                        self.edit_mark_ratio = Some(ratio);
                    }
                }
            }
            WaveformDrag::EditFade(_) => {
                self.active_drag = Some(drag);
                self.update_active_edit_fade(ratio);
                self.active_drag = None;
            }
            WaveformDrag::SelectionResize(_) => {
                self.active_drag = Some(drag);
                self.update_active_selection_resize(ratio);
                self.active_drag = None;
            }
            WaveformDrag::SelectionMove(_) => {
                self.active_drag = Some(drag);
                self.update_active_selection_move(ratio);
                self.active_drag = None;
            }
            WaveformDrag::PlaySelectionExport => {}
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn apply_play_selection_export_drag(&mut self, drag: radiant::widgets::DragHandleMessage) {
        match drag {
            radiant::widgets::DragHandleMessage::Started { .. } => {
                self.active_drag = Some(WaveformDrag::PlaySelectionExport);
            }
            radiant::widgets::DragHandleMessage::Moved { .. } => {}
            radiant::widgets::DragHandleMessage::Ended { .. } => {
                if matches!(self.active_drag, Some(WaveformDrag::PlaySelectionExport)) {
                    self.active_drag = None;
                }
            }
        }
    }

    fn update_active_edit_fade(&mut self, ratio: f32) {
        let Some(WaveformDrag::EditFade(drag)) = self.active_drag else {
            return;
        };
        let Some(selection) = self.edit_selection else {
            return;
        };
        self.edit_selection = Some(drag.apply(selection, ratio));
    }

    fn clear_edit_fade_silence(&mut self, handle: WaveformEditFadeHandle) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        let next = match handle {
            WaveformEditFadeHandle::InOuterStart => selection
                .fade_in()
                .map(|fade| selection.with_fade_in_and_mute(fade.length, fade.curve, 0.0)),
            WaveformEditFadeHandle::OutOuterEnd => selection
                .fade_out()
                .map(|fade| selection.with_fade_out_and_mute(fade.length, fade.curve, 0.0)),
            _ => None,
        };
        if let Some(next) = next {
            self.edit_selection = Some(next);
        }
    }
}
