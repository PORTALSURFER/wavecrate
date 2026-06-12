use super::segment_keys::segment_key_changed;
use crate::app_core::actions::{NativeAppModel, NativeColumnModel};
use crate::app_core::controller::AppController;
use crate::app_core::state::UiState;
use crate::app_core::ui_bridge::projection_cache::{DerivedProjectionState, UiProjectionCache};
use crate::app_core::ui_projection;

/// Refresh always-on non-segment metadata that is not covered by static keys.
pub(super) fn refresh_always_fields(model: &mut NativeAppModel, selected_column: usize) {
    model.selected_column = selected_column;
}

/// Refresh static non-segment app-model fields from current controller state.
pub(super) fn refresh_static_fields(model: &mut NativeAppModel, controller: &mut AppController) {
    model.transport_running = controller.is_playing();
    model.volume = controller.ui.volume.clamp(0.0, 1.0);
    model.sources = ui_projection::project_sources_model(controller);
    model.sources_label = format!("Sources ({})", model.sources.rows.len());
    model.focus_context = ui_projection::project_focus_context_model(controller.ui.focus.context);
    model.columns = [
        NativeColumnModel::new("Trash", controller.ui.browser.trash.len()),
        NativeColumnModel::new("Samples", controller.ui.browser.neutral.len()),
        NativeColumnModel::new("Keep", controller.ui.browser.keep.len()),
    ];
    model.update = ui_projection::project_update_model(&controller.ui);
}

/// Refresh the lightweight audio-chip fields consumed by the static top bar.
pub(super) fn refresh_audio_chip_fields(model: &mut NativeAppModel, ui: &UiState) {
    let chip = ui_projection::project_audio_engine_chip_model(ui);
    model.audio_engine.chip_state = chip.chip_state;
    model.audio_engine.chip_label = chip.chip_label;
}

/// Refresh transient non-segment overlays from current controller state.
pub(super) fn refresh_overlay_fields(model: &mut NativeAppModel, controller: &AppController) {
    let refresh_audio_engine = controller.ui.options_panel.open || model.options_panel.visible;
    if refresh_audio_engine {
        model.audio_engine = ui_projection::project_audio_engine_model(&controller.ui);
    }
    model.options_panel = ui_projection::project_options_panel_model(&controller.ui);
    model.progress_overlay = ui_projection::project_progress_overlay_model(&controller.ui);
    model.confirm_prompt = ui_projection::project_confirm_prompt_model(&controller.ui);
    model.drag_overlay = ui_projection::project_drag_overlay_model(&controller.ui);
}

/// Update static non-segment cache key and report whether it changed.
pub(super) fn update_static_key(
    cache: &mut UiProjectionCache,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(
        has_retained_model,
        &cache.non_segment_static_key,
        &derived.non_segment_static_key,
    );
    cache.non_segment_static_key = Some(derived.non_segment_static_key.clone());
    changed
}

/// Update the retained non-segment overlay key and return whether it changed.
pub(super) fn update_overlay_key(
    cache: &mut UiProjectionCache,
    derived: &DerivedProjectionState,
    has_retained_model: bool,
) -> bool {
    let changed = segment_key_changed(
        has_retained_model,
        &cache.non_segment_overlay_key,
        &derived.non_segment_overlay_key,
    );
    cache.non_segment_overlay_key = Some(derived.non_segment_overlay_key.clone());
    changed
}
