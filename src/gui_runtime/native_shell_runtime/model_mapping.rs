use super::*;
use prompt::{confirm_prompt_from_compat, confirm_prompt_to_compat};

fn retained_vec_from_compat<T, U>(value: runtime_contract::RetainedVec<T>) -> RetainedVec<U>
where
    T: Clone + Into<U>,
{
    value
        .as_slice()
        .iter()
        .cloned()
        .map(Into::into)
        .collect::<Vec<_>>()
        .into()
}

fn retained_vec_to_compat<T, U>(value: RetainedVec<T>) -> runtime_contract::RetainedVec<U>
where
    T: Clone + Into<U>,
{
    value
        .as_slice()
        .iter()
        .cloned()
        .map(Into::into)
        .collect::<Vec<_>>()
        .into()
}

mod audio;
mod browser;
mod prompt;

impl From<runtime_contract::AppModel> for AppModel {
    fn from(value: runtime_contract::AppModel) -> Self {
        let mut browser: BrowserPanelModel = value.browser.into();
        browser.sidebar_filters = value.sidebar_filters.clone();
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status,
            audio_engine: value.paired_device.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel,
            progress_overlay: value.progress_overlay,
            confirm_prompt: confirm_prompt_from_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay,
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources,
            browser,
            browser_chrome: value.browser_chrome.into(),
            map: value.map,
            waveform: value.waveform,
            waveform_chrome: value.waveform_chrome,
            update: value.update,
            focus_context: value.focus_context.into(),
        }
    }
}

impl From<AppModel> for runtime_contract::AppModel {
    fn from(value: AppModel) -> Self {
        let sidebar_filters = value.browser.sidebar_filters.clone();
        Self {
            title: value.title,
            backend_label: value.backend_label,
            sources_label: value.sources_label,
            status_text: value.status_text,
            status: value.status,
            paired_device: value.audio_engine.into(),
            browser_actions: value.browser_actions.into(),
            options_panel: value.options_panel,
            progress_overlay: value.progress_overlay,
            confirm_prompt: confirm_prompt_to_compat(value.confirm_prompt),
            drag_overlay: value.drag_overlay,
            columns: value.columns.map(Into::into),
            selected_column: value.selected_column,
            volume: value.volume,
            transport_running: value.transport_running,
            sources: value.sources,
            browser: value.browser.into(),
            sidebar_filters,
            browser_chrome: value.browser_chrome.into(),
            map: value.map,
            waveform: value.waveform,
            waveform_chrome: value.waveform_chrome,
            update: value.update,
            focus_context: value.focus_context.into(),
        }
    }
}

impl From<&AppModel> for runtime_contract::AppModel {
    fn from(value: &AppModel) -> Self {
        value.clone().into()
    }
}

pub(super) fn local_app_model_from_native_model(
    value: &AppModel,
) -> crate::app_core::native_shell::runtime_contract::AppModel {
    crate::app_core::native_shell::runtime_contract::AppModel {
        title: value.title.clone(),
        backend_label: value.backend_label.clone(),
        sources_label: value.sources_label.clone(),
        status_text: value.status_text.clone(),
        status: value.status.clone(),
        paired_device: value.audio_engine.clone().into(),
        browser_actions: value.browser_actions.clone().into(),
        options_panel: value.options_panel.clone(),
        progress_overlay: value.progress_overlay.clone(),
        confirm_prompt: local_confirm_prompt_from_native_model(&value.confirm_prompt),
        drag_overlay: value.drag_overlay.clone(),
        columns: value.columns.clone().map(Into::into),
        selected_column: value.selected_column,
        volume: value.volume,
        transport_running: value.transport_running,
        sources: local_sources_panel_from_native_model(&value.sources),
        browser: value.browser.clone().into(),
        sidebar_filters: value.browser.sidebar_filters.clone(),
        browser_chrome: value.browser_chrome.clone().into(),
        map: value.map.clone(),
        waveform: value.waveform.clone(),
        waveform_chrome: value.waveform_chrome.clone(),
        update: value.update.clone(),
        focus_context: value.focus_context.into(),
    }
}

fn local_confirm_prompt_from_native_model(
    value: &ConfirmPromptModel,
) -> crate::app_core::native_shell::runtime_contract::ConfirmPromptModel {
    crate::app_core::native_shell::runtime_contract::ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title.clone(),
        message: value.message.clone(),
        confirm_label: value.confirm_label.clone(),
        cancel_label: value.cancel_label.clone(),
        target_label: value.target_label.clone(),
        input_value: value.input_value.clone(),
        input_placeholder: value.input_placeholder.clone(),
        input_error: value.input_error.clone(),
    }
}

fn local_sources_panel_from_native_model(
    value: &SourcesPanelModel,
) -> crate::app_core::native_shell::runtime_contract::SourcesPanelModel {
    crate::app_core::native_shell::runtime_contract::SourcesPanelModel {
        header: value.header.clone(),
        search_query: value.search_query.clone(),
        active_folder_pane: value.active_folder_pane,
        upper_folder_pane: value.upper_folder_pane.clone(),
        lower_folder_pane: value.lower_folder_pane.clone(),
        tree_search_query: value.tree_search_query.clone(),
        show_all_items: value.show_all_items,
        can_toggle_show_all_items: value.can_toggle_show_all_items,
        flattened_view: value.flattened_view,
        can_toggle_flattened_view: value.can_toggle_flattened_view,
        selected_row: value.selected_row,
        loading_row: value.loading_row,
        mutation_busy_row: value.mutation_busy_row,
        focused_tree_row: value.focused_tree_row,
        rows: value.rows.clone(),
        tree_rows: value.tree_rows.clone(),
        tree_actions: value.tree_actions.clone(),
        recovery: value.recovery.clone(),
    }
}
