use super::*;

fn retained_vec_from_compat<T, U>(value: compat::RetainedVec<T>) -> RetainedVec<U>
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

fn retained_vec_to_compat<T, U>(value: RetainedVec<T>) -> compat::RetainedVec<U>
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

impl From<compat::FocusContextModel> for FocusContextModel {
    fn from(value: compat::FocusContextModel) -> Self {
        match value {
            compat::FocusContextModel::None => Self::None,
            compat::FocusContextModel::Timeline => Self::Waveform,
            compat::FocusContextModel::ContentList => Self::SampleBrowser,
            compat::FocusContextModel::NavigationTree => Self::SourceFolders,
            compat::FocusContextModel::NavigationList => Self::SourcesList,
        }
    }
}

impl From<FocusContextModel> for compat::FocusContextModel {
    fn from(value: FocusContextModel) -> Self {
        match value {
            FocusContextModel::None => Self::None,
            FocusContextModel::Waveform => Self::Timeline,
            FocusContextModel::SampleBrowser => Self::ContentList,
            FocusContextModel::SourceFolders => Self::NavigationTree,
            FocusContextModel::SourcesList => Self::NavigationList,
        }
    }
}

impl From<&SourcesPanelModel> for compat::SourcesPanelModel {
    fn from(value: &SourcesPanelModel) -> Self {
        value.clone()
    }
}

impl From<compat::BrowserPanelModel> for BrowserPanelModel {
    fn from(value: compat::BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_path_count: value.selected_item_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_recency_filters,
            marked_filter_active: value.marked_filter_active,
            tag_named_filter_active: value.derived_label_filter_active,
            tag_named_filter_negated: value.derived_label_filter_negated,
            sidebar_filters: Default::default(),
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            source_loading: value.data_loading,
            metadata_pending: value.metadata_pending,
            file_op_pending: value.mutation_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_sample_label: value.focused_item_label,
            tag_sidebar: value.pill_editor,
            anchor_visible_row: value.anchor_visible_row,
            rows: retained_vec_from_compat(value.rows),
        }
    }
}

impl From<BrowserPanelModel> for compat::BrowserPanelModel {
    fn from(value: BrowserPanelModel) -> Self {
        Self {
            visible_count: value.visible_count,
            selected_visible_row: value.selected_visible_row,
            autoscroll: value.autoscroll,
            view_start_row: value.view_start_row,
            selected_item_count: value.selected_path_count,
            search_query: value.search_query,
            active_rating_filters: value.active_rating_filters,
            active_recency_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            derived_label_filter_active: value.tag_named_filter_active,
            derived_label_filter_negated: value.tag_named_filter_negated,
            search_placeholder: value.search_placeholder,
            busy: value.busy,
            data_loading: value.source_loading,
            metadata_pending: value.metadata_pending,
            mutation_pending: value.file_op_pending,
            similarity_filtered: value.similarity_filtered,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            sort_label: value.sort_label,
            active_tab_label: value.active_tab_label,
            focused_item_label: value.focused_sample_label,
            pill_editor: value.tag_sidebar,
            anchor_visible_row: value.anchor_visible_row,
            rows: retained_vec_to_compat(value.rows),
        }
    }
}

impl From<&BrowserPanelModel> for compat::BrowserPanelModel {
    fn from(value: &BrowserPanelModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserChromeModel> for BrowserChromeModel {
    fn from(value: compat::BrowserChromeModel) -> Self {
        Self {
            samples_tab_label: value.items_tab_label,
            sample_column_label: value.item_column_label,
            map_tab_label: value.map_tab_label,
            tag_editor_label: value.pill_editor_label,
            search_prefix_label: value.search_prefix_label,
            search_placeholder: value.search_placeholder,
            activity_ready_label: value.activity_ready_label,
            activity_busy_label: value.activity_busy_label,
            sort_prefix_label: value.sort_prefix_label,
            sort_order_label: value.sort_order_label,
            similarity_toggle_label: value.similarity_toggle_label,
            item_count_label: value.item_count_label,
        }
    }
}

impl From<BrowserChromeModel> for compat::BrowserChromeModel {
    fn from(value: BrowserChromeModel) -> Self {
        Self {
            items_tab_label: value.samples_tab_label,
            item_column_label: value.sample_column_label,
            map_tab_label: value.map_tab_label,
            pill_editor_label: value.tag_editor_label,
            search_prefix_label: value.search_prefix_label,
            search_placeholder: value.search_placeholder,
            activity_ready_label: value.activity_ready_label,
            activity_busy_label: value.activity_busy_label,
            sort_prefix_label: value.sort_prefix_label,
            sort_order_label: value.sort_order_label,
            similarity_toggle_label: value.similarity_toggle_label,
            item_count_label: value.item_count_label,
        }
    }
}

impl From<&BrowserChromeModel> for compat::BrowserChromeModel {
    fn from(value: &BrowserChromeModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::BrowserActionsModel> for BrowserActionsModel {
    fn from(value: compat::BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_tag: value.can_edit_pills,
            can_normalize_focused_sample: value.can_process_focused_item,
            can_loop_crossfade_focused_sample: value.can_open_focused_item_flow,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            tag_sidebar_open: value.pill_editor_open,
        }
    }
}

impl From<BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: BrowserActionsModel) -> Self {
        Self {
            can_rename: value.can_rename,
            can_delete: value.can_delete,
            can_edit_pills: value.can_tag,
            can_process_focused_item: value.can_normalize_focused_sample,
            can_open_focused_item_flow: value.can_loop_crossfade_focused_sample,
            random_navigation_enabled: value.random_navigation_enabled,
            duplicate_cleanup_active: value.duplicate_cleanup_active,
            pill_editor_open: value.tag_sidebar_open,
        }
    }
}

impl From<&BrowserActionsModel> for compat::BrowserActionsModel {
    fn from(value: &BrowserActionsModel) -> Self {
        value.clone().into()
    }
}

impl From<compat::PairedPickerTargetModel> for AudioPickerTargetModel {
    fn from(value: compat::PairedPickerTargetModel) -> Self {
        match value {
            compat::PairedPickerTargetModel::PrimaryGroup => Self::OutputHost,
            compat::PairedPickerTargetModel::PrimaryItem => Self::OutputDevice,
            compat::PairedPickerTargetModel::PrimaryNumber => Self::OutputSampleRate,
            compat::PairedPickerTargetModel::SecondaryGroup => Self::InputHost,
            compat::PairedPickerTargetModel::SecondaryItem => Self::InputDevice,
            compat::PairedPickerTargetModel::SecondaryNumber => Self::InputSampleRate,
        }
    }
}

impl From<AudioPickerTargetModel> for compat::PairedPickerTargetModel {
    fn from(value: AudioPickerTargetModel) -> Self {
        match value {
            AudioPickerTargetModel::OutputHost => Self::PrimaryGroup,
            AudioPickerTargetModel::OutputDevice => Self::PrimaryItem,
            AudioPickerTargetModel::OutputSampleRate => Self::PrimaryNumber,
            AudioPickerTargetModel::InputHost => Self::SecondaryGroup,
            AudioPickerTargetModel::InputDevice => Self::SecondaryItem,
            AudioPickerTargetModel::InputSampleRate => Self::SecondaryNumber,
        }
    }
}

impl From<compat::PairedPickerValueModel> for AudioOptionValueModel {
    fn from(value: compat::PairedPickerValueModel) -> Self {
        match value {
            compat::PairedPickerValueModel::PrimaryGroup(value) => Self::OutputHost(value),
            compat::PairedPickerValueModel::PrimaryItem(value) => Self::OutputDevice(value),
            compat::PairedPickerValueModel::PrimaryNumber(value) => Self::OutputSampleRate(value),
            compat::PairedPickerValueModel::SecondaryGroup(value) => Self::InputHost(value),
            compat::PairedPickerValueModel::SecondaryItem(value) => Self::InputDevice(value),
            compat::PairedPickerValueModel::SecondaryNumber(value) => Self::InputSampleRate(value),
        }
    }
}

impl From<AudioOptionValueModel> for compat::PairedPickerValueModel {
    fn from(value: AudioOptionValueModel) -> Self {
        match value {
            AudioOptionValueModel::OutputHost(value) => Self::PrimaryGroup(value),
            AudioOptionValueModel::OutputDevice(value) => Self::PrimaryItem(value),
            AudioOptionValueModel::OutputSampleRate(value) => Self::PrimaryNumber(value),
            AudioOptionValueModel::InputHost(value) => Self::SecondaryGroup(value),
            AudioOptionValueModel::InputDevice(value) => Self::SecondaryItem(value),
            AudioOptionValueModel::InputSampleRate(value) => Self::SecondaryNumber(value),
        }
    }
}

fn audio_option_item_from_compat(value: compat::PairedPickerOptionModel) -> AudioOptionItemModel {
    AudioOptionItemModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

fn audio_option_item_to_compat(value: AudioOptionItemModel) -> compat::PairedPickerOptionModel {
    compat::PairedPickerOptionModel {
        label: value.label,
        selected: value.selected,
        value: value.value.into(),
    }
}

impl From<compat::PairedDevicePanelModel> for AudioEngineModel {
    fn from(value: compat::PairedDevicePanelModel) -> Self {
        Self {
            chip_state: value.status_state,
            chip_label: value.status_label,
            detail_label: value.detail_label,
            output_host: value.primary_group,
            output_device: value.primary_item,
            output_sample_rate: value.primary_number,
            input_host: value.secondary_group,
            input_device: value.secondary_item,
            input_sample_rate: value.secondary_number,
            active_picker: value.active_picker.map(Into::into),
            output_host_options: value
                .primary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_device_options: value
                .primary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            output_sample_rate_options: value
                .primary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_host_options: value
                .secondary_group_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_device_options: value
                .secondary_item_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
            input_sample_rate_options: value
                .secondary_number_options
                .into_iter()
                .map(audio_option_item_from_compat)
                .collect(),
        }
    }
}

impl From<AudioEngineModel> for compat::PairedDevicePanelModel {
    fn from(value: AudioEngineModel) -> Self {
        Self {
            status_state: value.chip_state,
            status_label: value.chip_label,
            detail_label: value.detail_label,
            primary_group: value.output_host,
            primary_item: value.output_device,
            primary_number: value.output_sample_rate,
            secondary_group: value.input_host,
            secondary_item: value.input_device,
            secondary_number: value.input_sample_rate,
            active_picker: value.active_picker.map(Into::into),
            primary_group_options: value
                .output_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_item_options: value
                .output_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            primary_number_options: value
                .output_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_group_options: value
                .input_host_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_item_options: value
                .input_device_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
            secondary_number_options: value
                .input_sample_rate_options
                .into_iter()
                .map(audio_option_item_to_compat)
                .collect(),
        }
    }
}

impl From<&AudioEngineModel> for compat::PairedDevicePanelModel {
    fn from(value: &AudioEngineModel) -> Self {
        value.clone().into()
    }
}

impl From<&OptionsPanelModel> for compat::OptionsPanelModel {
    fn from(value: &OptionsPanelModel) -> Self {
        value.clone()
    }
}

impl From<compat::ConfirmPromptKind> for ConfirmPromptKind {
    fn from(value: compat::ConfirmPromptKind) -> Self {
        match value {
            compat::ConfirmPromptKind::DestructiveOperation => Self::DestructiveEdit,
            compat::ConfirmPromptKind::RenameContent => Self::BrowserRename,
            compat::ConfirmPromptKind::RenameNavigationItem => Self::FolderRename,
            compat::ConfirmPromptKind::CreateNavigationItem => Self::FolderCreate,
            compat::ConfirmPromptKind::RestoreRetainedItems => Self::RestoreRetainedFolderDeletes,
            compat::ConfirmPromptKind::PurgeRetainedItems => Self::PurgeRetainedFolderDeletes,
            compat::ConfirmPromptKind::EditConfiguration => Self::OptionsDefaultIdentifier,
        }
    }
}

impl From<ConfirmPromptKind> for compat::ConfirmPromptKind {
    fn from(value: ConfirmPromptKind) -> Self {
        match value {
            ConfirmPromptKind::DestructiveEdit => Self::DestructiveOperation,
            ConfirmPromptKind::BrowserRename => Self::RenameContent,
            ConfirmPromptKind::FolderRename => Self::RenameNavigationItem,
            ConfirmPromptKind::FolderCreate => Self::CreateNavigationItem,
            ConfirmPromptKind::RestoreRetainedFolderDeletes => Self::RestoreRetainedItems,
            ConfirmPromptKind::PurgeRetainedFolderDeletes => Self::PurgeRetainedItems,
            ConfirmPromptKind::OptionsDefaultIdentifier => Self::EditConfiguration,
        }
    }
}

fn confirm_prompt_from_compat(value: compat::ConfirmPromptModel) -> ConfirmPromptModel {
    ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

fn confirm_prompt_to_compat(value: ConfirmPromptModel) -> compat::ConfirmPromptModel {
    compat::ConfirmPromptModel {
        visible: value.visible,
        kind: value.kind.map(Into::into),
        title: value.title,
        message: value.message,
        confirm_label: value.confirm_label,
        cancel_label: value.cancel_label,
        target_label: value.target_label,
        input_value: value.input_value,
        input_placeholder: value.input_placeholder,
        input_error: value.input_error,
    }
}

impl From<&WaveformPanelModel> for compat::WaveformPanelModel {
    fn from(value: &WaveformPanelModel) -> Self {
        value.clone()
    }
}

impl From<&WaveformChromeModel> for compat::WaveformChromeModel {
    fn from(value: &WaveformChromeModel) -> Self {
        value.clone()
    }
}

impl From<compat::AppModel> for AppModel {
    fn from(value: compat::AppModel) -> Self {
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

impl From<AppModel> for compat::AppModel {
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

impl From<&AppModel> for compat::AppModel {
    fn from(value: &AppModel) -> Self {
        value.clone().into()
    }
}

pub(super) fn local_app_model_from_native_model(
    value: &AppModel,
) -> crate::compat_app_contract::AppModel {
    crate::compat_app_contract::AppModel {
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
) -> crate::compat_app_contract::ConfirmPromptModel {
    crate::compat_app_contract::ConfirmPromptModel {
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
) -> crate::compat_app_contract::SourcesPanelModel {
    crate::compat_app_contract::SourcesPanelModel {
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
