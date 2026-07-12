use super::*;

impl AppController {
    /// Save the current waveform selection or marked slice batch into the browser.
    ///
    /// This shares the same export path used by waveform drag-drop so keyboard and
    /// pointer workflows produce identical files and status updates.
    pub(crate) fn save_waveform_selection_or_slices_to_browser(
        &mut self,
        keep_source_focused: bool,
    ) -> Result<(), String> {
        self.save_waveform_selection_or_slices_to_browser_with_tag(keep_source_focused, None)
    }

    fn save_waveform_selection_or_slices_to_browser_with_tag(
        &mut self,
        keep_source_focused: bool,
        target_tag: Option<Rating>,
    ) -> Result<(), String> {
        if !self.ui.waveform.slices.is_empty() {
            self.start_waveform_slice_batch_export_with_tag(target_tag)?;
            return Ok(());
        }
        self.save_waveform_selection_to_browser_with_tag(keep_source_focused, target_tag)
    }

    /// Save the current waveform selection or slices and surface any failure via status UI.
    pub(crate) fn save_waveform_selection_or_slices_to_browser_action(
        &mut self,
        keep_source_focused: bool,
    ) {
        self.save_waveform_selection_or_slices_to_browser_action_with_tag(
            keep_source_focused,
            None,
        );
    }

    /// Save the current waveform selection or slices with an explicit tag.
    pub(crate) fn save_waveform_selection_or_slices_to_browser_action_with_tag(
        &mut self,
        keep_source_focused: bool,
        target_tag: Option<Rating>,
    ) {
        if let Err(err) = self
            .save_waveform_selection_or_slices_to_browser_with_tag(keep_source_focused, target_tag)
        {
            self.set_error_status(err);
        }
    }

    pub(crate) fn save_waveform_selection_to_browser(
        &mut self,
        keep_source_focused: bool,
    ) -> Result<(), String> {
        self.save_waveform_selection_to_browser_with_tag(keep_source_focused, None)
    }

    fn save_waveform_selection_to_browser_with_tag(
        &mut self,
        keep_source_focused: bool,
        target_tag: Option<Rating>,
    ) -> Result<(), String> {
        let selection = self.active_waveform_selection_for_export()?;
        let folder_override = self.selection_export_folder_override();
        let request_id = self.runtime.jobs.next_selection_export_request_id();
        self.begin_pending_sample_creation_transaction(
            crate::app::controller::history::PendingHistoryTransactionKey::SelectionExport {
                request_id,
            },
            "Saved selection clip",
        );
        self.queue_selection_export_job(SelectionExportJob::Clip {
            request_id,
            snapshot: self.capture_selection_export_snapshot(selection, target_tag)?,
            destination: SelectionClipDestination::Browser {
                keep_source_focused,
                folder_override,
            },
        });
        self.record_waveform_selection_export_flash();
        self.set_status("Saving selection clip...", StatusTone::Busy);
        Ok(())
    }

    fn selection_export_folder_override(&self) -> Option<PathBuf> {
        self.selection_state
            .ctx
            .selected_source
            .as_ref()
            .zip(self.sample_view.wav.loaded_audio.as_ref())
            .is_some_and(|(selected, audio)| selected == &audio.source_id)
            .then(|| {
                self.ui.sources.folders.focused.and_then(|idx| {
                    self.ui
                        .sources
                        .folders
                        .rows
                        .get(idx)
                        .map(|row| row.path.clone())
                })
            })
            .flatten()
            .filter(|path| !path.as_os_str().is_empty())
    }
}
