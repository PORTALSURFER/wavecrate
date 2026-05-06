use super::*;

pub(super) fn automation_node_id_from_generic(
    value: gui_automation::AutomationNodeId,
) -> AutomationNodeId {
    gui_automation::AutomationNodeId(automation_node_id_string_from_generic(value.0))
}

pub(super) fn automation_node_id_string_from_generic(node_id: String) -> String {
    match node_id.as_str() {
        "browser.tab.items" => String::from("browser.tab.samples"),
        "browser.pill_editor" => String::from("browser.tag_sidebar"),
        "browser.pill_editor.input" => String::from("browser.tag_sidebar.input"),
        "browser.pill_editor.exclusive.0" => String::from("browser.tag_sidebar.playback.loop"),
        "browser.pill_editor.exclusive.1" => String::from("browser.tag_sidebar.playback.one_shot"),
        _ => {
            if let Some(suffix) = node_id.strip_prefix("browser.pill_editor.option.") {
                format!("browser.tag_sidebar.normal_tag.{suffix}")
            } else if let Some(suffix) = node_id.strip_prefix("browser.pill_editor.create.") {
                format!("browser.tag_sidebar.create_tag.{suffix}")
            } else {
                node_id
            }
        }
    }
}

impl From<gui_automation::AutomationRole> for AutomationRole {
    fn from(value: gui_automation::AutomationRole) -> Self {
        match value {
            gui_automation::AutomationRole::Root => Self::Root,
            gui_automation::AutomationRole::Group => Self::Group,
            gui_automation::AutomationRole::Panel => Self::Panel,
            gui_automation::AutomationRole::Toolbar => Self::Toolbar,
            gui_automation::AutomationRole::TabList => Self::TabList,
            gui_automation::AutomationRole::Tab => Self::Tab,
            gui_automation::AutomationRole::Button => Self::Button,
            gui_automation::AutomationRole::SearchField => Self::SearchField,
            gui_automation::AutomationRole::Slider => Self::Slider,
            gui_automation::AutomationRole::Row => Self::Row,
            gui_automation::AutomationRole::Table => Self::Table,
            gui_automation::AutomationRole::TimelineRegion => Self::WaveformRegion,
            gui_automation::AutomationRole::SpatialCanvas => Self::MapCanvas,
            gui_automation::AutomationRole::SpatialPoint => Self::MapPoint,
            gui_automation::AutomationRole::Readout => Self::Readout,
            gui_automation::AutomationRole::Dialog => Self::Dialog,
        }
    }
}

pub(super) fn automation_bounds_from_generic(
    value: gui_automation::AutomationBounds,
) -> AutomationBounds {
    AutomationBounds {
        x: value.x,
        y: value.y,
        width: value.width,
        height: value.height,
    }
}

impl From<gui_automation::AutomationNodeSnapshot> for AutomationNodeSnapshot {
    fn from(value: gui_automation::AutomationNodeSnapshot) -> Self {
        Self {
            id: automation_node_id_from_generic(value.id),
            role: value.role.into(),
            label: value.label,
            bounds: automation_bounds_from_generic(value.bounds),
            value: value.value,
            enabled: value.enabled,
            selected: value.selected,
            available_actions: value
                .available_actions
                .into_iter()
                .map(automation_action_id_from_generic)
                .collect(),
            metadata: automation_metadata_from_generic(value.metadata),
            children: value.children.into_iter().map(Into::into).collect(),
        }
    }
}

pub(super) fn automation_action_id_from_generic(action_id: String) -> String {
    match action_id.as_str() {
        "open_primary_group_picker" => String::from("open_audio_output_host_picker"),
        "open_primary_item_picker" => String::from("open_audio_output_device_picker"),
        "open_primary_number_picker" => String::from("open_audio_output_sample_rate_picker"),
        "open_secondary_group_picker" => String::from("open_audio_input_host_picker"),
        "open_secondary_item_picker" => String::from("open_audio_input_device_picker"),
        "open_secondary_number_picker" => String::from("open_audio_input_sample_rate_picker"),
        "set_primary_group" => String::from("set_audio_output_host"),
        "set_primary_item" => String::from("set_audio_output_device"),
        "set_primary_number" => String::from("set_audio_output_sample_rate"),
        "set_secondary_group" => String::from("set_audio_input_host"),
        "set_secondary_item" => String::from("set_audio_input_device"),
        "set_secondary_number" => String::from("set_audio_input_sample_rate"),
        "focus_spatial_content_item" => String::from("focus_map_sample"),
        "focus_browser_pill_editor_input" => String::from("focus_browser_tag_sidebar_input"),
        "set_browser_pill_editor_input" => String::from("set_browser_tag_sidebar_input"),
        "commit_browser_pill_editor_input" => String::from("commit_browser_tag_sidebar_input"),
        "toggle_browser_pill_editor" => String::from("toggle_browser_tag_sidebar"),
        "toggle_browser_pill_editor_primary_action" => {
            String::from("toggle_browser_tag_sidebar_auto_rename")
        }
        "toggle_browser_pill_option" => String::from("toggle_browser_sidebar_normal_tag"),
        "toggle_browser_derived_label_filter" => String::from("toggle_browser_tag_named_filter"),
        _ => action_id,
    }
}

pub(super) fn automation_metadata_from_generic(
    mut metadata: BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    if let Some(value) = metadata.remove("focused_item_label") {
        metadata.insert(String::from("focused_sample_label"), value);
    }
    if let Some(value) = metadata.remove("option_pill_labels") {
        metadata.insert(String::from("normal_tag_labels"), value);
    }
    if let Some(value) = metadata.remove("pill_state") {
        metadata.insert(String::from("tag_state"), value);
    }
    if let Some(value) = metadata.remove("pill_id") {
        metadata.insert(String::from("tag_id"), value);
    }
    metadata
}

impl From<gui_automation::GuiAutomationSnapshot> for GuiAutomationSnapshot {
    fn from(value: gui_automation::GuiAutomationSnapshot) -> Self {
        Self {
            schema_version: value.schema_version,
            viewport_width: value.viewport_width,
            viewport_height: value.viewport_height,
            root: value.root.into(),
        }
    }
}

impl From<runtime_contract::NativeMotionModel> for NativeMotionModel {
    fn from(value: runtime_contract::NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli,
            waveform_slices: value.waveform_slices.into_iter().collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli,
            waveform_edit_fade_in_end_milli: value.waveform_edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: value.waveform_edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: value.waveform_edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: value.waveform_edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: value.waveform_edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: value.waveform_edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: value.waveform_edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: value.waveform_edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: value.waveform_edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: value.waveform_edit_fade_out_curve_milli,
            waveform_loop_enabled: value.waveform_loop_enabled,
            waveform_loop_lock_enabled: value.waveform_loop_lock_enabled,
            waveform_cursor_milli: value.waveform_cursor_milli,
            waveform_playhead_milli: value.waveform_playhead_milli,
            waveform_playhead_micros: value.waveform_playhead_micros,
            waveform_view_start_milli: value.waveform_view_start_milli,
            waveform_view_end_milli: value.waveform_view_end_milli,
            waveform_view_start_micros: value.waveform_view_start_micros,
            waveform_view_end_micros: value.waveform_view_end_micros,
            waveform_view_start_nanos: value.waveform_view_start_nanos,
            waveform_view_end_nanos: value.waveform_view_end_nanos,
            waveform_tempo_label: value.waveform_tempo_label,
            waveform_zoom_label: value.waveform_zoom_label,
            waveform_loaded_label: value.waveform_loaded_label,
            waveform_loading: value.waveform_loading,
            waveform_image_signature: value.waveform_image_signature,
            waveform_transport_hint: value.waveform_transport_hint,
            waveform_compare_anchor_available: value.waveform_compare_anchor_available,
            waveform_compare_anchor_label: value.waveform_compare_anchor_label,
            waveform_channel_view: value.waveform_channel_view,
            waveform_normalized_audition_enabled: value.waveform_normalized_audition_enabled,
            waveform_bpm_snap_enabled: value.waveform_bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: value.waveform_relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: value.waveform_transient_snap_enabled,
            waveform_transient_markers_enabled: value.waveform_transient_markers_enabled,
            waveform_slice_mode_enabled: value.waveform_slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: value
                .waveform_exact_duplicate_cleanup_available,
            status_right: value.status_right,
        }
    }
}

impl From<NativeMotionModel> for runtime_contract::NativeMotionModel {
    fn from(value: NativeMotionModel) -> Self {
        Self {
            transport_running: value.transport_running,
            map_active: value.map_active,
            active_rating_filters: value.active_rating_filters,
            active_playback_age_filters: value.active_playback_age_filters,
            marked_filter_active: value.marked_filter_active,
            waveform_selection_milli: value.waveform_selection_milli,
            waveform_slices: value.waveform_slices.into_iter().collect(),
            waveform_selection_export_flash_nonce: value.waveform_selection_export_flash_nonce,
            waveform_selection_export_failure_flash_nonce: value
                .waveform_selection_export_failure_flash_nonce,
            waveform_edit_selection_apply_flash_nonce: value
                .waveform_edit_selection_apply_flash_nonce,
            waveform_edit_selection_milli: value.waveform_edit_selection_milli,
            waveform_edit_fade_in_end_milli: value.waveform_edit_fade_in_end_milli,
            waveform_edit_fade_in_end_micros: value.waveform_edit_fade_in_end_micros,
            waveform_edit_fade_in_mute_start_milli: value.waveform_edit_fade_in_mute_start_milli,
            waveform_edit_fade_in_mute_start_micros: value.waveform_edit_fade_in_mute_start_micros,
            waveform_edit_fade_in_curve_milli: value.waveform_edit_fade_in_curve_milli,
            waveform_edit_fade_out_start_milli: value.waveform_edit_fade_out_start_milli,
            waveform_edit_fade_out_start_micros: value.waveform_edit_fade_out_start_micros,
            waveform_edit_fade_out_mute_end_milli: value.waveform_edit_fade_out_mute_end_milli,
            waveform_edit_fade_out_mute_end_micros: value.waveform_edit_fade_out_mute_end_micros,
            waveform_edit_fade_out_curve_milli: value.waveform_edit_fade_out_curve_milli,
            waveform_loop_enabled: value.waveform_loop_enabled,
            waveform_loop_lock_enabled: value.waveform_loop_lock_enabled,
            waveform_cursor_milli: value.waveform_cursor_milli,
            waveform_playhead_milli: value.waveform_playhead_milli,
            waveform_playhead_micros: value.waveform_playhead_micros,
            waveform_view_start_milli: value.waveform_view_start_milli,
            waveform_view_end_milli: value.waveform_view_end_milli,
            waveform_view_start_micros: value.waveform_view_start_micros,
            waveform_view_end_micros: value.waveform_view_end_micros,
            waveform_view_start_nanos: value.waveform_view_start_nanos,
            waveform_view_end_nanos: value.waveform_view_end_nanos,
            waveform_tempo_label: value.waveform_tempo_label,
            waveform_zoom_label: value.waveform_zoom_label,
            waveform_loaded_label: value.waveform_loaded_label,
            waveform_loading: value.waveform_loading,
            waveform_image_signature: value.waveform_image_signature,
            waveform_transport_hint: value.waveform_transport_hint,
            waveform_compare_anchor_available: value.waveform_compare_anchor_available,
            waveform_compare_anchor_label: value.waveform_compare_anchor_label,
            waveform_channel_view: value.waveform_channel_view,
            waveform_normalized_audition_enabled: value.waveform_normalized_audition_enabled,
            waveform_bpm_snap_enabled: value.waveform_bpm_snap_enabled,
            waveform_relative_bpm_grid_enabled: value.waveform_relative_bpm_grid_enabled,
            waveform_transient_snap_enabled: value.waveform_transient_snap_enabled,
            waveform_transient_markers_enabled: value.waveform_transient_markers_enabled,
            waveform_slice_mode_enabled: value.waveform_slice_mode_enabled,
            waveform_exact_duplicate_cleanup_available: value
                .waveform_exact_duplicate_cleanup_available,
            status_right: value.status_right,
        }
    }
}

pub(crate) fn capture_gui_automation_snapshot(
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> NativeGuiAutomationSnapshot {
    let local_model = local_app_model_from_native_model(model);
    let viewport = Vector2::new(viewport[0].max(1.0), viewport[1].max(1.0));
    let style = StyleTokens::for_viewport_width(viewport.x);
    let mut runtime = ShellLayoutRuntime::default();
    let layout = ShellLayout::build_with_style_and_runtime(viewport, &style, &mut runtime);
    let mut shell_state = NativeShellState::new();
    shell_state.sync_from_model(&local_model);
    local_automation_snapshot_from_native_shell(
        shell_state.automation_snapshot(&layout, &local_model),
    )
}

pub(super) fn local_automation_snapshot_from_native_shell(
    value: GuiAutomationSnapshot,
) -> NativeGuiAutomationSnapshot {
    NativeGuiAutomationSnapshot {
        schema_version: value.schema_version,
        viewport_width: value.viewport_width,
        viewport_height: value.viewport_height,
        root: local_automation_node_from_native_shell(value.root),
    }
}

pub(super) fn local_automation_node_from_native_shell(
    value: AutomationNodeSnapshot,
) -> AutomationNodeSnapshot {
    AutomationNodeSnapshot {
        id: automation_node_id_from_generic(value.id),
        role: value.role,
        label: value.label,
        bounds: value.bounds,
        value: value.value,
        enabled: value.enabled,
        selected: value.selected,
        available_actions: value
            .available_actions
            .into_iter()
            .map(automation_action_id_from_generic)
            .collect(),
        metadata: automation_metadata_from_generic(value.metadata),
        children: value
            .children
            .into_iter()
            .map(local_automation_node_from_native_shell)
            .collect(),
    }
}

#[cfg(test)]
pub(crate) fn capture_native_shell_shot_snapshot(
    name: impl Into<String>,
    viewport: [f32; 2],
    model: &NativeAppModel,
) -> impl serde::Serialize {
    let local_model = local_app_model_from_native_model(model);
    crate::app_core::native_shell::runtime_contract::capture_native_shell_shot_snapshot(
        name,
        viewport,
        &local_model,
    )
}
