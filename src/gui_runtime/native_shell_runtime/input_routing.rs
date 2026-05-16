use super::*;

fn focus_context_from_radiant(focus: RadiantFocusSurface) -> FocusContext {
    match focus {
        RadiantFocusSurface::None => FocusContext::None,
        RadiantFocusSurface::Timeline => FocusContext::Waveform,
        RadiantFocusSurface::ContentList => FocusContext::SampleBrowser,
        RadiantFocusSurface::NavigationTree => FocusContext::SourceFolders,
        RadiantFocusSurface::NavigationList => FocusContext::SourcesList,
    }
}

/// Resolve Wavecrate focus from the projected model, falling back to Radiant focus.
pub(super) fn wavecrate_focus_context(
    model: &runtime_contract::AppModel,
    focus: RadiantFocusSurface,
) -> FocusContext {
    match model.focus_context {
        runtime_contract::FocusContextModel::None => focus_context_from_radiant(focus),
        runtime_contract::FocusContextModel::Timeline => FocusContext::Waveform,
        runtime_contract::FocusContextModel::ContentList => FocusContext::SampleBrowser,
        runtime_contract::FocusContextModel::NavigationTree => FocusContext::SourceFolders,
        runtime_contract::FocusContextModel::NavigationList => FocusContext::SourcesList,
    }
}

pub(super) fn keypress_from_radiant(press: RadiantKeyPress) -> KeyPress {
    KeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

pub(super) fn keypress_to_radiant(press: KeyPress) -> RadiantKeyPress {
    RadiantKeyPress {
        key: press.key,
        command: press.command,
        shift: press.shift,
        alt: press.alt,
    }
}

/// Resolve a retained pointer press into a Wavecrate compatibility action.
pub(super) fn action_from_retained_pointer(
    layout: &ShellLayout,
    model: &runtime_contract::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<runtime_contract::UiAction> {
    route_modal_and_chrome_actions(layout, model, shell_state, point)
        .or_else(|| route_browser_or_folder_row(layout, model, shell_state, point))
        .or_else(|| route_shell_background(layout, model, shell_state, point))
}

/// Route pointer presses through modal, chrome, toolbar, and action surfaces.
fn route_modal_and_chrome_actions(
    layout: &ShellLayout,
    model: &runtime_contract::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<runtime_contract::UiAction> {
    if let Some(action) = shell_state.prompt_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.progress_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.options_panel_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.sidebar_filter_dropdown_action_at_point(layout, model, point)
    {
        return Some(action);
    }
    if shell_state.sidebar_filter_dropdown_visible() {
        if shell_state.sidebar_filter_dropdown_contains_point(layout, model, point) {
            return None;
        }
        shell_state.close_sidebar_filter_dropdown();
        return Some(runtime_contract::UiAction::FocusBrowserPanel);
    }
    if model.options_panel.visible {
        if shell_state.options_panel_contains_point_live(layout, model, point) {
            return None;
        }
    }
    if let Some(action) = shell_state.status_options_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.top_bar_update_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.top_bar_volume_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_tab_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.map_content_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_action_at_point(layout, model, point, false) {
        return Some(action);
    }
    if let Some(action) = shell_state.source_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.folder_header_action_at_point(layout, model, point) {
        return Some(action);
    }
    let motion_model = runtime_contract::NativeMotionModel::from_app_model(model);
    shell_state
        .waveform_toolbar_action_at_point_with_motion_and_modifiers(
            layout,
            &motion_model,
            point,
            false,
        )
        .or_else(|| {
            shell_state.waveform_toolbar_action_at_point_with_modifiers(layout, model, point, false)
        })
}

/// Route pointer presses that target browser rows or folder rows.
fn route_browser_or_folder_row(
    layout: &ShellLayout,
    model: &runtime_contract::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<runtime_contract::UiAction> {
    if let Some(action) = shell_state.browser_row_similarity_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(visible_row) = shell_state.browser_row_at_point(layout, model, point) {
        return Some(runtime_contract::UiAction::FocusBrowserRow { visible_row });
    }
    if let Some((pane, index)) = shell_state.folder_row_disclosure_at_point(layout, model, point) {
        return Some(folder_row_disclosure_action(model, pane, index));
    }
    shell_state
        .folder_row_at_point(layout, model, point)
        .map(|(pane, index)| folder_row_body_action(model, pane, index))
}

/// Route pointer presses that land on the shell background areas.
fn route_shell_background(
    layout: &ShellLayout,
    model: &runtime_contract::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<runtime_contract::UiAction> {
    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => route_sidebar_background(layout, model, shell_state, point),
        ShellNodeKind::WaveformCard => {
            if layout.waveform_plot.contains(point) {
                Some(waveform_cursor_action_from_point(layout, model, point))
            } else {
                Some(runtime_contract::UiAction::FocusWaveformPanel)
            }
        }
        ShellNodeKind::TopBar => Some(runtime_contract::UiAction::ToggleTransport),
        ShellNodeKind::BrowserPanel | ShellNodeKind::BrowserTabs | ShellNodeKind::BrowserTable => {
            Some(runtime_contract::UiAction::FocusBrowserPanel)
        }
        ShellNodeKind::StatusBar => Some(runtime_contract::UiAction::FocusLoadedContentInList),
        _ => None,
    }
}

/// Route pointer presses that land in sidebar background space.
fn route_sidebar_background(
    layout: &ShellLayout,
    model: &runtime_contract::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<runtime_contract::UiAction> {
    if let Some((_pane, index)) = shell_state.source_row_at_point(layout, model, point) {
        return Some(runtime_contract::UiAction::FocusSourceRow { index });
    }
    if let Some((pane, index)) = shell_state.folder_row_disclosure_at_point(layout, model, point) {
        return Some(folder_row_disclosure_action(model, pane, index));
    }
    if let Some((pane, index)) = shell_state.folder_row_at_point(layout, model, point) {
        return Some(folder_row_body_action(model, pane, index));
    }
    shell_state.sidebar_focus_action_at_point(layout, model, point)
}

/// Return the folder-row action for a disclosure target.
fn folder_row_disclosure_action(
    model: &runtime_contract::AppModel,
    pane: FolderPaneIdModel,
    index: usize,
) -> runtime_contract::UiAction {
    let Some(row) = folder_row_for_pointer_action(model, pane, index) else {
        return runtime_contract::UiAction::FocusFolderRow { index };
    };
    if matches!(
        row.kind,
        runtime_contract::FolderRowKind::CreateDraft | runtime_contract::FolderRowKind::RenameDraft
    ) {
        return runtime_contract::UiAction::FocusFolderCreateInput;
    }
    let source_index = row.backing_index.unwrap_or(index);
    if folder_row_disclosure_toggles_expansion(model.sources.folder_pane(pane), index) {
        runtime_contract::UiAction::ToggleFolderRowExpanded {
            index: source_index,
        }
    } else {
        runtime_contract::UiAction::FocusFolderRow {
            index: source_index,
        }
    }
}

/// Return the folder-row action for a row body target.
fn folder_row_body_action(
    model: &runtime_contract::AppModel,
    pane: FolderPaneIdModel,
    index: usize,
) -> runtime_contract::UiAction {
    let Some(row) = folder_row_for_pointer_action(model, pane, index) else {
        return runtime_contract::UiAction::FocusFolderRow { index };
    };
    if matches!(
        row.kind,
        runtime_contract::FolderRowKind::CreateDraft | runtime_contract::FolderRowKind::RenameDraft
    ) {
        return runtime_contract::UiAction::FocusFolderCreateInput;
    }
    runtime_contract::UiAction::FocusFolderRow {
        index: row.backing_index.unwrap_or(index),
    }
}

/// Resolve the folder row model addressed by retained pointer routing.
fn folder_row_for_pointer_action(
    model: &runtime_contract::AppModel,
    pane: FolderPaneIdModel,
    index: usize,
) -> Option<&runtime_contract::FolderRowModel> {
    let pane_row = model.sources.folder_pane(pane).tree_rows.get(index);
    let flat_active_row = (pane == model.sources.active_folder_pane)
        .then(|| model.sources.tree_rows.get(index))
        .flatten();
    flat_active_row
        .filter(|row| {
            matches!(
                row.kind,
                runtime_contract::FolderRowKind::CreateDraft
                    | runtime_contract::FolderRowKind::RenameDraft
            )
        })
        .or(pane_row)
        .or(flat_active_row)
}

/// Return whether a folder-row disclosure should expand or collapse the row.
fn folder_row_disclosure_toggles_expansion(
    pane_model: &runtime_contract::FolderPaneModel,
    index: usize,
) -> bool {
    let Some(row) = pane_model.tree_rows.get(index) else {
        return false;
    };
    row.has_children
        && !row.is_root
        && !matches!(
            row.kind,
            runtime_contract::FolderRowKind::CreateDraft
                | runtime_contract::FolderRowKind::RenameDraft
        )
        && pane_model.tree_search_query.trim().is_empty()
}

/// Translate a waveform-plot pointer location into an absolute cursor action.
fn waveform_cursor_action_from_point(
    layout: &ShellLayout,
    model: &runtime_contract::AppModel,
    point: Point,
) -> runtime_contract::UiAction {
    let x_ratio = if layout.waveform_plot.width() <= 0.0 {
        0.0
    } else {
        ((point.x - layout.waveform_plot.min.x) / layout.waveform_plot.width()).clamp(0.0, 1.0)
    };
    let viewport = model.waveform.viewport();
    let start = viewport.start_nanos.min(viewport.end_nanos);
    let end = viewport.start_nanos.max(viewport.end_nanos);
    let span = end.saturating_sub(start);
    runtime_contract::UiAction::SetWaveformCursorPrecise {
        position_nanos: start.saturating_add(((span as f32) * x_ratio).round() as u32),
    }
}
