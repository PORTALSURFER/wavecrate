//! Shared BPM text-entry helpers.
//!
//! The BPM editor shares its selection and buffer lifecycle with the other
//! single-line text fields, but the parsing and pointer hit-testing are split
//! out here so the native runtime stays readable.

use super::*;

#[path = "pointer.rs"]
mod pointer;
#[path = "state.rs"]
mod state;

#[allow(unused_imports)]
pub(crate) use state::{
    bpm_tenths_from_value, parse_waveform_bpm_input, sanitize_waveform_bpm_insert,
};

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(super) fn folder_inline_edit_row(
        &self,
    ) -> Option<&crate::compat_app_contract::FolderRowModel> {
        self.model
            .sources
            .tree_rows
            .iter()
            .find(|row| row.kind == crate::compat_app_contract::FolderRowKind::RenameDraft)
            .or_else(|| {
                self.model
                    .sources
                    .tree_rows
                    .iter()
                    .find(|row| row.kind == crate::compat_app_contract::FolderRowKind::CreateDraft)
            })
    }

    pub(super) fn sync_text_editor_visual_state_for_target(&mut self, target: TextInputTarget) {
        pointer::sync_text_editor_visual_state_for_target(self, target);
    }

    pub(super) fn handle_browser_search_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        pointer::handle_browser_search_pointer_press(self, layout, point, extend_selection)
    }

    pub(super) fn handle_waveform_bpm_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        pointer::handle_waveform_bpm_pointer_press(self, layout, point, extend_selection)
    }

    pub(super) fn handle_folder_create_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        pointer::handle_folder_create_pointer_press(self, layout, point, extend_selection)
    }

    pub(super) fn process_text_input_drag(&mut self, point: Point) -> bool {
        pointer::process_text_input_drag(self, point)
    }

    pub(super) fn sync_text_input_target(&mut self) {
        state::sync_text_input_target(self);
    }

    pub(super) fn current_text_value(&self) -> Option<String> {
        state::current_text_value(self)
    }

    pub(super) fn set_text_value(&mut self, value: String) -> bool {
        state::set_text_value(self, value)
    }

    pub(super) fn append_text(&mut self, appended: &str) -> bool {
        state::append_text(self, appended)
    }

    pub(super) fn waveform_bpm_text_from_model(&self) -> String {
        state::waveform_bpm_text_from_model(self)
    }
}
