use radiant::gui::types::Vector2;

use super::{
    MIN_VISIBLE_FRAMES, WAVEFORM_WIDTH, WaveformEditFadeHandle, WaveformInteraction,
    WaveformSelectionKind, WaveformState, WaveformViewport,
    interaction::{
        WaveformDrag, WaveformEditFadeDrag, WaveformPanDrag, WaveformSelectionDrag,
        WaveformSelectionMoveDrag, WaveformSelectionResizeDrag,
    },
};

impl WaveformState {
    pub(in crate::gui_app) fn apply_interaction(&mut self, interaction: WaveformInteraction) {
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
            WaveformInteraction::Frame => {
                self.play_selection_flash_frames =
                    self.play_selection_flash_frames.saturating_sub(1);
            }
        }
    }

    pub(super) fn absolute_ratio_from_visible(&self, visible_ratio: f32) -> f32 {
        self.viewport.absolute_ratio_from_visible(
            self.file.frames.max(1),
            MIN_VISIBLE_FRAMES,
            visible_ratio,
        )
    }

    fn handle_wheel(&mut self, delta: Vector2, anchor_ratio: f32) {
        if delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON {
            self.pan_by_visible_fraction(delta.x / WAVEFORM_WIDTH as f32);
            return;
        }
        if delta.y < -f32::EPSILON {
            self.zoom_around_anchor(0.82, anchor_ratio);
        } else if delta.y > f32::EPSILON {
            self.zoom_around_anchor(1.22, anchor_ratio);
        }
    }

    fn zoom_around_anchor(&mut self, factor: f32, anchor_ratio: f32) {
        let total = self.file.frames.max(1);
        self.viewport =
            self.viewport
                .zoom_around_anchor(total, MIN_VISIBLE_FRAMES, factor, anchor_ratio);
    }

    fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.file.frames.max(1);
        self.viewport = self
            .viewport
            .pan_by_visible_fraction(total, MIN_VISIBLE_FRAMES, fraction);
    }

    fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.file.frames.max(1);
        self.viewport =
            self.viewport
                .with_offset_fraction(total, MIN_VISIBLE_FRAMES, offset_fraction);
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
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn set_selection_for_drag(&mut self, drag: WaveformSelectionDrag) {
        let range =
            wavecrate::selection::SelectionRange::new(drag.anchor_ratio, drag.current_ratio);
        match drag.kind {
            WaveformSelectionKind::Play => {
                self.play_mark_ratio = Some(drag.anchor_ratio);
                self.play_selection = Some(range);
            }
            WaveformSelectionKind::Edit => {
                self.edit_mark_ratio = Some(drag.anchor_ratio);
                self.edit_selection = Some(range);
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
            WaveformEditFadeHandle::FadeInOuterStart => selection
                .fade_in()
                .map(|fade| selection.with_fade_in_and_mute(fade.length, fade.curve, 0.0)),
            WaveformEditFadeHandle::FadeOutOuterEnd => selection
                .fade_out()
                .map(|fade| selection.with_fade_out_and_mute(fade.length, fade.curve, 0.0)),
            _ => None,
        };
        if let Some(next) = next {
            self.edit_selection = Some(next);
        }
    }

    fn update_active_selection_resize(&mut self, ratio: f32) {
        let Some(WaveformDrag::SelectionResize(drag)) = self.active_drag else {
            return;
        };
        let Some(selection) = self.selection_for_kind(drag.kind) else {
            return;
        };
        let selection = drag.apply(selection, ratio);
        match drag.kind {
            WaveformSelectionKind::Play => {
                self.play_mark_ratio = Some(selection.start());
                self.play_selection = Some(selection);
            }
            WaveformSelectionKind::Edit => {
                self.edit_mark_ratio = Some(selection.start());
                self.edit_selection = Some(selection);
            }
        }
    }

    fn update_active_selection_move(&mut self, ratio: f32) {
        let Some(WaveformDrag::SelectionMove(drag)) = self.active_drag else {
            return;
        };
        let selection = drag.apply(ratio);
        match drag.kind {
            WaveformSelectionKind::Play => {
                self.play_mark_ratio = Some(selection.start());
                self.play_selection = Some(selection);
            }
            WaveformSelectionKind::Edit => {
                self.edit_mark_ratio = Some(selection.start());
                self.edit_selection = Some(selection);
            }
        }
    }

    fn update_active_pan(&mut self, drag: WaveformPanDrag, visible_ratio: f32) {
        let total = self.file.frames.max(1);
        let viewport = drag.viewport.clamp(total, MIN_VISIBLE_FRAMES);
        let visible = viewport.visible_items();
        if visible >= total {
            return;
        }
        let delta = ((visible_ratio - drag.anchor_visible_ratio) * visible as f32).round() as isize;
        let start = viewport.start.saturating_add_signed(-delta);
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total, MIN_VISIBLE_FRAMES);
    }

    fn selection_for_kind(
        &self,
        kind: WaveformSelectionKind,
    ) -> Option<wavecrate::selection::SelectionRange> {
        match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        }
    }
}
