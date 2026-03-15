//! Staged native app-model projection assembly.
//!
//! This module owns the full-model projection pipeline used by the retained
//! native bridge. It keeps the one-shot derived inputs, core panel
//! materialization, and overlay/chrome assembly together so the root
//! `native_shell` facade can stay small.

use super::*;

/// Immutable projection inputs derived once per app-model projection.
pub(crate) struct ProjectAppModelDerivedInputs {
    /// Selected triage column index used by status and top-level model metadata.
    selected_column: usize,
    /// Transport-running state projected into the top-level app model.
    transport_running: bool,
    /// Flat status text mirrored in the top-level app model.
    status_text: String,
    /// Triage/browser item counts used to project top-level column metadata.
    column_counts: [usize; 3],
    /// Master output volume clamped into the normalized `0.0..=1.0` range.
    clamped_volume: f32,
    /// Current logical focus context projected into native input routing.
    focus_context: FocusContextModel,
}

/// Core panel models that may require mutable controller access during projection.
pub(crate) struct ProjectAppModelCoreModels {
    /// Source and folder panel model.
    sources: SourcesPanelModel,
    /// Status bar segment model.
    status: StatusBarModel,
    /// Browser action availability model.
    browser_actions: BrowserActionsModel,
    /// Map panel model.
    map: MapPanelModel,
    /// Waveform panel model.
    waveform: WaveformPanelModel,
    /// Browser panel model (frame metadata + row window).
    pub(crate) browser: BrowserPanelModel,
}

/// Overlay and chrome models that depend on projected core model metadata.
pub(crate) struct ProjectAppModelOverlayAndChromeModels {
    /// Update surface model.
    update: UpdatePanelModel,
    /// Options-panel overlay model.
    options_panel: OptionsPanelModel,
    /// Progress overlay model.
    progress_overlay: ProgressOverlayModel,
    /// Confirm prompt overlay model.
    confirm_prompt: ConfirmPromptModel,
    /// Drag feedback overlay model.
    drag_overlay: DragOverlayModel,
    /// Browser chrome/toolbar labels.
    browser_chrome: BrowserChromeModel,
    /// Waveform header chrome labels.
    waveform_chrome: WaveformChromeModel,
}

/// Project the full native app model from the current controller/UI snapshot.
///
/// This is the top-level projection entry used by the bridge full-pull path.
/// It stages derived scalar inputs, then materializes core panels and overlays
/// in a deterministic order to keep cross-panel labels and counts in sync.
pub(crate) fn project_app_model(controller: &mut AppController) -> AppModel {
    let call = PROJECT_APP_MODEL_CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    let derived_inputs = derive_project_app_model_inputs(controller);
    if call <= 12 {
        info!(
            call,
            selected_column = derived_inputs.selected_column,
            status_len = derived_inputs.status_text.len(),
            visible_browser_rows = controller.ui.browser.viewport.visible.len(),
            "native shell: project_app_model start"
        );
    }
    let core_models = materialize_project_app_model_core(controller, &derived_inputs);
    let overlay_and_chrome_models = materialize_project_app_model_overlay_and_chrome(
        &controller.ui,
        core_models.browser.visible_count,
    );
    let app_model =
        assemble_project_app_model(derived_inputs, core_models, overlay_and_chrome_models);
    if call <= 12 {
        info!(
            call,
            browser_visible = app_model.browser.visible_count,
            status_center_len = app_model.status.center.len(),
            transport_running = app_model.transport_running,
            "native shell: project_app_model complete"
        );
    }
    app_model
}

/// Derive scalar projection inputs shared across staged app-model materialization.
pub(crate) fn derive_project_app_model_inputs(
    controller: &AppController,
) -> ProjectAppModelDerivedInputs {
    ProjectAppModelDerivedInputs {
        selected_column: selected_column_index(&controller.ui),
        transport_running: controller.is_playing(),
        status_text: controller.ui.status.text.clone(),
        column_counts: [
            controller.ui.browser.trash.len(),
            controller.ui.browser.neutral.len(),
            controller.ui.browser.keep.len(),
        ],
        clamped_volume: controller.ui.volume.clamp(0.0, 1.0),
        focus_context: project_focus_context_model(controller.ui.focus.context),
    }
}

/// Materialize core panel models for the staged app-model projection pipeline.
pub(crate) fn materialize_project_app_model_core(
    controller: &mut AppController,
    derived_inputs: &ProjectAppModelDerivedInputs,
) -> ProjectAppModelCoreModels {
    ProjectAppModelCoreModels {
        sources: project_sources_model(&controller.ui),
        status: project_status_model(controller, derived_inputs.selected_column),
        browser_actions: project_browser_actions_model(&controller.ui),
        map: project_map_model(controller),
        waveform: project_waveform_model(controller),
        browser: project_browser_model(controller),
    }
}

/// Materialize overlays/chrome after core panels are projected.
pub(crate) fn materialize_project_app_model_overlay_and_chrome(
    ui: &UiState,
    browser_visible_count: usize,
) -> ProjectAppModelOverlayAndChromeModels {
    ProjectAppModelOverlayAndChromeModels {
        update: project_update_model(ui),
        options_panel: project_options_panel_model(ui),
        progress_overlay: project_progress_overlay_model(ui),
        confirm_prompt: project_confirm_prompt_model(ui),
        drag_overlay: project_drag_overlay_model(ui),
        browser_chrome: project_browser_chrome_model(ui, browser_visible_count),
        waveform_chrome: project_waveform_chrome_model(ui),
    }
}

/// Assemble the final native app model from staged projection outputs.
pub(crate) fn assemble_project_app_model(
    derived_inputs: ProjectAppModelDerivedInputs,
    core_models: ProjectAppModelCoreModels,
    overlay_and_chrome_models: ProjectAppModelOverlayAndChromeModels,
) -> AppModel {
    AppModel {
        title: String::from("Sempal"),
        backend_label: String::from("backend: native_vello"),
        sources_label: format!("Sources ({})", core_models.sources.rows.len()),
        status_text: derived_inputs.status_text,
        status: core_models.status,
        browser_actions: core_models.browser_actions,
        options_panel: overlay_and_chrome_models.options_panel,
        progress_overlay: overlay_and_chrome_models.progress_overlay,
        confirm_prompt: overlay_and_chrome_models.confirm_prompt,
        drag_overlay: overlay_and_chrome_models.drag_overlay,
        columns: [
            ColumnModel::new("Trash", derived_inputs.column_counts[0]),
            ColumnModel::new("Samples", derived_inputs.column_counts[1]),
            ColumnModel::new("Keep", derived_inputs.column_counts[2]),
        ],
        selected_column: derived_inputs.selected_column,
        volume: derived_inputs.clamped_volume,
        transport_running: derived_inputs.transport_running,
        sources: core_models.sources,
        browser: core_models.browser,
        browser_chrome: overlay_and_chrome_models.browser_chrome,
        map: core_models.map,
        waveform: core_models.waveform,
        waveform_chrome: overlay_and_chrome_models.waveform_chrome,
        update: overlay_and_chrome_models.update,
        focus_context: derived_inputs.focus_context,
    }
}
