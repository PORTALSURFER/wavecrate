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

/// Resolve Sempal focus from the projected model, falling back to Radiant focus.
pub(super) fn sempal_focus_context(
    model: &compat::AppModel,
    focus: RadiantFocusSurface,
) -> FocusContext {
    match model.focus_context {
        compat::FocusContextModel::None => focus_context_from_radiant(focus),
        compat::FocusContextModel::Timeline => FocusContext::Waveform,
        compat::FocusContextModel::ContentList => FocusContext::SampleBrowser,
        compat::FocusContextModel::NavigationTree => FocusContext::SourceFolders,
        compat::FocusContextModel::NavigationList => FocusContext::SourcesList,
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

/// Convert one Radiant widget input event into the Sempal retained input shape.
pub(super) fn retained_input_from_widget_input(input: WidgetInput) -> RetainedCanvasInput {
    match input {
        WidgetInput::PointerMove { position } => RetainedCanvasInput::PointerMove { position },
        WidgetInput::PointerPress { position, button } => {
            RetainedCanvasInput::PointerPress { position, button }
        }
        WidgetInput::PointerRelease { position, button } => {
            RetainedCanvasInput::PointerRelease { position, button }
        }
        WidgetInput::FocusChanged(focused) => RetainedCanvasInput::FocusChanged(focused),
        WidgetInput::KeyPress(key) => RetainedCanvasInput::KeyPress(key),
        WidgetInput::Character(character) => RetainedCanvasInput::Character(character),
    }
}

/// Resolve a retained pointer press into a Sempal compatibility action.
pub(super) fn action_from_retained_pointer(
    layout: &ShellLayout,
    model: &compat::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<compat::UiAction> {
    route_modal_and_chrome_actions(layout, model, shell_state, point)
        .or_else(|| route_browser_or_folder_row(layout, model, shell_state, point))
        .or_else(|| route_shell_background(layout, model, shell_state, point))
}

/// Route pointer presses through modal, chrome, toolbar, and action surfaces.
fn route_modal_and_chrome_actions(
    layout: &ShellLayout,
    model: &compat::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<compat::UiAction> {
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
        return Some(compat::UiAction::FocusBrowserPanel);
    }
    if model.options_panel.visible {
        if shell_state.options_panel_contains_point_live(layout, model, point) {
            return None;
        }
        return Some(compat::UiAction::CloseOptionsPanel);
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
    let motion_model = compat::NativeMotionModel::from_app_model(model);
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
    model: &compat::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<compat::UiAction> {
    if let Some(action) = shell_state.browser_row_similarity_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(visible_row) = shell_state.browser_row_at_point(layout, model, point) {
        return Some(compat::UiAction::FocusBrowserRow { visible_row });
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
    model: &compat::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<compat::UiAction> {
    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => route_sidebar_background(layout, model, shell_state, point),
        ShellNodeKind::WaveformCard => {
            if layout.waveform_plot.contains(point) {
                Some(waveform_cursor_action_from_point(layout, model, point))
            } else {
                Some(compat::UiAction::FocusWaveformPanel)
            }
        }
        ShellNodeKind::TopBar => Some(compat::UiAction::ToggleTransport),
        ShellNodeKind::BrowserPanel | ShellNodeKind::BrowserTabs | ShellNodeKind::BrowserTable => {
            Some(compat::UiAction::FocusBrowserPanel)
        }
        ShellNodeKind::StatusBar => Some(compat::UiAction::FocusLoadedContentInList),
        _ => None,
    }
}

/// Route pointer presses that land in sidebar background space.
fn route_sidebar_background(
    layout: &ShellLayout,
    model: &compat::AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<compat::UiAction> {
    if let Some((_pane, index)) = shell_state.source_row_at_point(layout, model, point) {
        return Some(compat::UiAction::FocusSourceRow { index });
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
    model: &compat::AppModel,
    pane: FolderPaneIdModel,
    index: usize,
) -> compat::UiAction {
    let Some(row) = folder_row_for_pointer_action(model, pane, index) else {
        return compat::UiAction::FocusFolderRow { index };
    };
    if matches!(
        row.kind,
        compat::FolderRowKind::CreateDraft | compat::FolderRowKind::RenameDraft
    ) {
        return compat::UiAction::FocusFolderCreateInput;
    }
    let source_index = row.backing_index.unwrap_or(index);
    if folder_row_disclosure_toggles_expansion(model.sources.folder_pane(pane), index) {
        compat::UiAction::ToggleFolderRowExpanded {
            index: source_index,
        }
    } else {
        compat::UiAction::FocusFolderRow {
            index: source_index,
        }
    }
}

/// Return the folder-row action for a row body target.
fn folder_row_body_action(
    model: &compat::AppModel,
    pane: FolderPaneIdModel,
    index: usize,
) -> compat::UiAction {
    let Some(row) = folder_row_for_pointer_action(model, pane, index) else {
        return compat::UiAction::FocusFolderRow { index };
    };
    if matches!(
        row.kind,
        compat::FolderRowKind::CreateDraft | compat::FolderRowKind::RenameDraft
    ) {
        return compat::UiAction::FocusFolderCreateInput;
    }
    compat::UiAction::FocusFolderRow {
        index: row.backing_index.unwrap_or(index),
    }
}

/// Resolve the folder row model addressed by retained pointer routing.
fn folder_row_for_pointer_action(
    model: &compat::AppModel,
    pane: FolderPaneIdModel,
    index: usize,
) -> Option<&compat::FolderRowModel> {
    let pane_row = model.sources.folder_pane(pane).tree_rows.get(index);
    let flat_active_row = (pane == model.sources.active_folder_pane)
        .then(|| model.sources.tree_rows.get(index))
        .flatten();
    flat_active_row
        .filter(|row| {
            matches!(
                row.kind,
                compat::FolderRowKind::CreateDraft | compat::FolderRowKind::RenameDraft
            )
        })
        .or(pane_row)
        .or(flat_active_row)
}

/// Return whether a folder-row disclosure should expand or collapse the row.
fn folder_row_disclosure_toggles_expansion(
    pane_model: &compat::FolderPaneModel,
    index: usize,
) -> bool {
    let Some(row) = pane_model.tree_rows.get(index) else {
        return false;
    };
    row.has_children
        && !row.is_root
        && !matches!(
            row.kind,
            compat::FolderRowKind::CreateDraft | compat::FolderRowKind::RenameDraft
        )
        && pane_model.tree_search_query.trim().is_empty()
}

/// Translate a waveform-plot pointer location into an absolute cursor action.
fn waveform_cursor_action_from_point(
    layout: &ShellLayout,
    model: &compat::AppModel,
    point: Point,
) -> compat::UiAction {
    let x_ratio = if layout.waveform_plot.width() <= 0.0 {
        0.0
    } else {
        ((point.x - layout.waveform_plot.min.x) / layout.waveform_plot.width()).clamp(0.0, 1.0)
    };
    let viewport = model.waveform.viewport();
    let start = viewport.start_nanos.min(viewport.end_nanos);
    let end = viewport.start_nanos.max(viewport.end_nanos);
    let span = end.saturating_sub(start);
    compat::UiAction::SetWaveformCursorPrecise {
        position_nanos: start.saturating_add(((span as f32) * x_ratio).round() as u32),
    }
}
