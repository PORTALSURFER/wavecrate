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
pub(in crate::gui_runtime::native_vello) use state::{
    bpm_tenths_from_value, parse_waveform_bpm_input, sanitize_waveform_bpm_insert,
};

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(in crate::gui_runtime::native_vello) fn folder_inline_edit_row(
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

    pub(in crate::gui_runtime::native_vello) fn sync_text_editor_visual_state_for_target(
        &mut self,
        target: TextInputTarget,
    ) {
        pointer::sync_text_editor_visual_state_for_target(self, target);
    }

    pub(in crate::gui_runtime::native_vello) fn handle_browser_search_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        pointer::handle_browser_search_pointer_press(self, layout, point, extend_selection)
    }

    pub(in crate::gui_runtime::native_vello) fn handle_waveform_bpm_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        pointer::handle_waveform_bpm_pointer_press(self, layout, point, extend_selection)
    }

    pub(in crate::gui_runtime::native_vello) fn handle_folder_create_pointer_press(
        &mut self,
        layout: &ShellLayout,
        point: Point,
        extend_selection: bool,
    ) -> bool {
        pointer::handle_folder_create_pointer_press(self, layout, point, extend_selection)
    }

    pub(in crate::gui_runtime::native_vello) fn process_text_input_drag(
        &mut self,
        point: Point,
    ) -> bool {
        pointer::process_text_input_drag(self, point)
    }

    pub(in crate::gui_runtime::native_vello) fn sync_text_input_target(&mut self) {
        state::sync_text_input_target(self);
    }

    pub(in crate::gui_runtime::native_vello) fn current_text_value(&self) -> Option<String> {
        state::current_text_value(self)
    }

    pub(in crate::gui_runtime::native_vello) fn set_text_value(&mut self, value: String) -> bool {
        state::set_text_value(self, value)
    }

    pub(in crate::gui_runtime::native_vello) fn append_text(&mut self, appended: &str) -> bool {
        state::append_text(self, appended)
    }

    pub(in crate::gui_runtime::native_vello) fn waveform_bpm_text_from_model(&self) -> String {
        state::waveform_bpm_text_from_model(self)
    }
}
