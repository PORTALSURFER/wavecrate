use super::*;

impl AppController {
    /// Apply edit-selection fades to disk and clear preview fades.
    pub(crate) fn commit_edit_selection_fades(&mut self) -> Result<bool, String> {
        let Some(selection) = self.ui.waveform.edit_selection else {
            return Ok(false);
        };
        if !selection.has_edit_effects() {
            return Ok(false);
        }
        if !cfg!(test) {
            self.queue_selection_edit_commit(
                "Applied edit fades",
                format!(
                    "Applied edit fades {}",
                    self.selection_target()?.relative_path.display()
                ),
                true,
                false,
                true,
                SelectionEditWorkerOp::ApplySelectionFades { selection },
            )?;
            return Ok(true);
        }
        let result = self.apply_selection_edit("Applied edit fades", true, |buffer| {
            apply_selection_fades(SelectionFadeRequest {
                samples: &mut buffer.samples,
                channels: buffer.channels,
                sample_rate: buffer.sample_rate,
                start_frame: buffer.start_frame,
                end_frame: buffer.end_frame,
                selection_gain: selection.gain(),
                fade_in: selection.fade_in(),
                fade_out: selection.fade_out(),
            });
            Ok(())
        });
        self.finish_edit_selection_fade_commit(selection, result)
    }

    /// Cancel preview fades for the edit selection without writing audio.
    pub(crate) fn cancel_edit_selection_fades(&mut self) -> bool {
        let Some(selection) = self.ui.waveform.edit_selection else {
            return false;
        };
        if !selection.has_edit_effects() {
            return false;
        }
        self.clear_edit_selection_effects(selection);
        true
    }

    /// Emit one success token so UI projections can flash the edit selection.
    pub(crate) fn record_edit_selection_apply_flash(&mut self) {
        self.ui.waveform.edit_selection_apply_flash_nonce = self
            .ui
            .waveform
            .edit_selection_apply_flash_nonce
            .wrapping_add(1);
    }

    fn finish_edit_selection_fade_commit(
        &mut self,
        selection: SelectionRange,
        result: Result<(), String>,
    ) -> Result<bool, String> {
        match result {
            Ok(()) => {
                self.clear_edit_selection_effects(selection);
                self.record_edit_selection_apply_flash();
                Ok(true)
            }
            Err(err) => {
                self.set_status(err.clone(), StatusTone::Error);
                Err(err)
            }
        }
    }

    fn clear_edit_selection_effects(&mut self, selection: SelectionRange) {
        let cleared = selection.clear_fades().with_gain(1.0);
        self.selection_state.edit_range.set_range(Some(cleared));
        self.apply_edit_selection(Some(cleared));
    }
}
