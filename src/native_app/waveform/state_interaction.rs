use super::{
    MIN_VISIBLE_FRAMES, WAVEFORM_WIDTH, WaveformEditFadeHandle, WaveformInteraction,
    WaveformSelectionKind, WaveformState,
    interaction::{
        WaveformDrag, WaveformEditFadeDrag, WaveformEditFadeOuterGainDrag, WaveformEditGainDrag,
        WaveformPanDrag, WaveformSampleSlideDrag, WaveformSelectionDrag, WaveformSelectionMoveDrag,
        WaveformSelectionResizeDrag,
    },
};

const LIVE_SELECTION_PREVIEW_STEPS_PER_PIXEL: f32 = 2.0;

impl WaveformState {
    pub(in crate::native_app) fn apply_interaction(&mut self, interaction: WaveformInteraction) {
        if !self.has_loaded_sample() && !matches!(interaction, WaveformInteraction::Frame) {
            self.active_drag = None;
            return;
        }
        match interaction {
            WaveformInteraction::Wheel {
                delta,
                anchor_ratio,
                expand_silence_margin,
            } => {
                self.zoom_anchor_ratio = anchor_ratio;
                self.handle_wheel(delta, anchor_ratio, expand_silence_margin);
            }
            WaveformInteraction::ZoomToPlaySelection => {
                self.zoom_to_play_selection();
            }
            WaveformInteraction::SlidePlaySelection { direction } => {
                self.active_drag = None;
                self.slide_play_selection_by_width(direction);
            }
            WaveformInteraction::ZoomFull => {
                self.zoom_full();
            }
            WaveformInteraction::ZoomOut {
                expand_silence_margin,
            } => {
                self.zoom_out(expand_silence_margin);
            }
            WaveformInteraction::RememberPointerLocation { position } => {
                self.context_menu_pointer_position = position
                    .x
                    .is_finite()
                    .then_some(position)
                    .filter(|position| position.y.is_finite());
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
                        self.play_mark_ratio = None;
                        self.play_selection = None;
                        self.play_selection_flash_frames = 0;
                        self.play_selection_denied_flash_frames = 0;
                    }
                    WaveformSelectionKind::Edit => {
                        self.edit_selection_flash_frames = 0;
                        self.edit_selection_denied_flash_frames = 0;
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
            WaveformInteraction::BeginEditFadeOuterGain {
                handle,
                vertical_ratio,
            } => {
                let Some(selection) = self.edit_selection else {
                    return;
                };
                self.active_drag = Some(WaveformDrag::EditFadeOuterGain(
                    WaveformEditFadeOuterGainDrag::new(handle),
                ));
                self.update_active_edit_fade_outer_gain(selection, vertical_ratio);
            }
            WaveformInteraction::UpdateEditFadeOuterGain { vertical_ratio } => {
                self.update_active_edit_fade_outer_gain_from_current(vertical_ratio);
            }
            WaveformInteraction::FinishEditFadeOuterGain { vertical_ratio } => {
                self.finish_active_edit_fade_outer_gain(vertical_ratio);
            }
            WaveformInteraction::BeginEditGain { pointer_y } => {
                let Some(selection) = self.edit_selection else {
                    return;
                };
                self.active_drag = Some(WaveformDrag::EditGain(WaveformEditGainDrag::new(
                    pointer_y, selection,
                )));
            }
            WaveformInteraction::UpdateEditGain { pointer_y } => {
                self.update_active_edit_gain(pointer_y);
            }
            WaveformInteraction::FinishEditGain { pointer_y } => {
                self.finish_active_edit_gain(pointer_y);
            }
            WaveformInteraction::ClearEditFadeSilence { handle } => {
                self.clear_edit_fade_silence(handle);
                self.active_drag = None;
            }
            WaveformInteraction::SelectSimilarSection { selection } => {
                self.active_drag = None;
                self.set_edit_selection_range(selection);
                self.flash_edit_selection();
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
                let allow_out_of_bounds = self.viewport.extends_beyond_audio(self.file.frames);
                self.active_drag = Some(WaveformDrag::SelectionResize(
                    WaveformSelectionResizeDrag::new(kind, edge, selection, allow_out_of_bounds),
                ));
                self.update_active_selection_resize(ratio, false);
            }
            WaveformInteraction::BeginSelectionMove {
                kind,
                visible_ratio,
            } => {
                let Some(selection) = self.selection_for_kind(kind) else {
                    return;
                };
                let ratio = self.absolute_ratio_from_visible(visible_ratio);
                let allow_out_of_bounds = self.viewport.extends_beyond_audio(self.file.frames);
                self.active_drag = Some(WaveformDrag::SelectionMove(
                    WaveformSelectionMoveDrag::new(kind, ratio, selection, allow_out_of_bounds),
                ));
                self.update_active_selection_move(ratio, false);
            }
            WaveformInteraction::BeginSampleSlide { visible_ratio } => {
                self.active_drag = Some(WaveformDrag::SampleSlide(WaveformSampleSlideDrag::new(
                    visible_ratio,
                    self.viewport,
                )));
                self.pending_sample_slide_frame_offset = None;
            }
            WaveformInteraction::UpdateSampleSlide { visible_ratio } => {
                self.update_active_sample_slide(visible_ratio);
            }
            WaveformInteraction::FinishSampleSlide { visible_ratio } => {
                self.finish_active_sample_slide(visible_ratio);
            }
            WaveformInteraction::BeginPan { visible_ratio } => {
                self.active_drag = Some(WaveformDrag::Pan(WaveformPanDrag::new(
                    visible_ratio,
                    self.viewport
                        .clamp_to_current_domain(self.file.frames, MIN_VISIBLE_FRAMES),
                )));
            }
            WaveformInteraction::UpdateSelection { visible_ratio } => {
                self.update_active_drag(quantized_live_selection_visible_ratio(visible_ratio));
            }
            WaveformInteraction::FinishSelection { visible_ratio } => {
                self.finish_active_drag(visible_ratio);
            }
            WaveformInteraction::DragPlaySelectionExport(drag) => {
                self.apply_play_selection_export_drag(drag);
            }
            WaveformInteraction::DragLoadedSample(_) => {}
            WaveformInteraction::Frame => {
                self.play_selection_flash_frames =
                    self.play_selection_flash_frames.saturating_sub(1);
                self.edit_selection_flash_frames =
                    self.edit_selection_flash_frames.saturating_sub(1);
                self.play_selection_denied_flash_frames =
                    self.play_selection_denied_flash_frames.saturating_sub(1);
                self.edit_selection_denied_flash_frames =
                    self.edit_selection_denied_flash_frames.saturating_sub(1);
                self.copy_flash_frames = self.copy_flash_frames.saturating_sub(1);
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
                if drag.moved() {
                    self.set_selection_for_drag(drag, false);
                }
            }
            WaveformDrag::EditFade(_) => {
                self.update_active_edit_fade(ratio);
            }
            WaveformDrag::EditFadeOuterGain(_) => {}
            WaveformDrag::EditGain(_) => {}
            WaveformDrag::SelectionResize(_) => {
                self.update_active_selection_resize(ratio, false);
            }
            WaveformDrag::SelectionMove(_) => {
                self.update_active_selection_move(ratio, false);
            }
            WaveformDrag::PlaySelectionExport => {}
            WaveformDrag::SampleSlide(_) => {
                self.update_active_sample_slide(visible_ratio);
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
                if drag.moved() {
                    let kind = drag.kind;
                    self.set_selection_for_drag(drag, true);
                    if kind == WaveformSelectionKind::Play {
                        self.clear_similar_sections();
                        self.record_current_play_selection_mark();
                    }
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
                        self.edit_mark_ratio = None;
                    }
                }
            }
            WaveformDrag::EditFade(_) => {
                self.active_drag = Some(drag);
                self.update_active_edit_fade(ratio);
                self.active_drag = None;
            }
            WaveformDrag::EditFadeOuterGain(_) => {
                self.active_drag = None;
            }
            WaveformDrag::EditGain(_) => {
                self.active_drag = None;
            }
            WaveformDrag::SelectionResize(drag) => {
                self.active_drag = Some(WaveformDrag::SelectionResize(drag));
                self.update_active_selection_resize(ratio, true);
                self.active_drag = None;
                if drag.kind == WaveformSelectionKind::Play {
                    self.clear_similar_sections();
                    self.record_current_play_selection_mark();
                }
            }
            WaveformDrag::SelectionMove(drag) => {
                self.active_drag = Some(WaveformDrag::SelectionMove(drag));
                self.update_active_selection_move(ratio, true);
                self.active_drag = None;
                if drag.kind == WaveformSelectionKind::Play {
                    self.clear_similar_sections();
                    self.record_current_play_selection_mark();
                }
            }
            WaveformDrag::PlaySelectionExport => {}
            WaveformDrag::SampleSlide(drag) => {
                self.pending_sample_slide_frame_offset = Some(drag.frame_offset(visible_ratio));
            }
            WaveformDrag::Pan(drag) => {
                self.update_active_pan(drag, visible_ratio);
            }
        }
    }

    fn apply_play_selection_export_drag(&mut self, drag: radiant::widgets::DragHandleMessage) {
        if drag.is_started() {
            self.active_drag = Some(WaveformDrag::PlaySelectionExport);
        } else if drag.is_finished()
            && matches!(self.active_drag, Some(WaveformDrag::PlaySelectionExport))
        {
            self.active_drag = None;
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

    fn update_active_edit_fade_outer_gain(
        &mut self,
        selection: wavecrate::selection::SelectionRange,
        vertical_ratio: f32,
    ) {
        let Some(WaveformDrag::EditFadeOuterGain(drag)) = self.active_drag else {
            return;
        };
        self.edit_selection = Some(drag.apply(selection, vertical_ratio));
    }

    fn update_active_edit_fade_outer_gain_from_current(&mut self, vertical_ratio: f32) {
        let Some(selection) = self.edit_selection else {
            return;
        };
        self.update_active_edit_fade_outer_gain(selection, vertical_ratio);
    }

    fn finish_active_edit_fade_outer_gain(&mut self, vertical_ratio: f32) {
        let Some(drag @ WaveformDrag::EditFadeOuterGain(_)) = self.active_drag else {
            return;
        };
        self.active_drag = Some(drag);
        self.update_active_edit_fade_outer_gain_from_current(vertical_ratio);
        self.active_drag = None;
    }

    fn update_active_edit_gain(&mut self, pointer_y: f32) {
        let Some(WaveformDrag::EditGain(drag)) = self.active_drag else {
            return;
        };
        self.edit_selection = Some(drag.apply(pointer_y));
    }

    fn finish_active_edit_gain(&mut self, pointer_y: f32) {
        let Some(drag @ WaveformDrag::EditGain(_)) = self.active_drag else {
            return;
        };
        self.active_drag = Some(drag);
        self.update_active_edit_gain(pointer_y);
        self.active_drag = None;
    }

    fn update_active_sample_slide(&mut self, visible_ratio: f32) {
        let Some(WaveformDrag::SampleSlide(drag)) = self.active_drag else {
            return;
        };
        self.pending_sample_slide_frame_offset = Some(drag.frame_offset(visible_ratio));
    }

    fn finish_active_sample_slide(&mut self, visible_ratio: f32) {
        let Some(drag @ WaveformDrag::SampleSlide(_)) = self.active_drag.take() else {
            return;
        };
        self.active_drag = Some(drag);
        self.update_active_sample_slide(visible_ratio);
        self.active_drag = None;
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

fn quantized_live_selection_visible_ratio(visible_ratio: f32) -> f32 {
    if !visible_ratio.is_finite() {
        return visible_ratio;
    }
    let steps = WAVEFORM_WIDTH as f32 * LIVE_SELECTION_PREVIEW_STEPS_PER_PIXEL;
    (visible_ratio * steps).round() / steps
}
