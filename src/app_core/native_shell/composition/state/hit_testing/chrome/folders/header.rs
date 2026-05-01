use self::sempal_crate::app as native_model;
use super::*;
use crate as sempal_crate;
use native_model::FolderPaneIdModel;

impl NativeShellState {
    /// Return the folder-visibility toggle button rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_visibility_toggle_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        folder_header_layout(layout, model, model.sources.active_folder_pane)
            .visibility_toggle_button
            .map(|button| button.rect)
    }

    /// Return the folder-flatten toggle button rect for tests.
    #[cfg(test)]
    pub(crate) fn folder_flatten_toggle_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        folder_header_layout(layout, model, model.sources.active_folder_pane)
            .flatten_toggle_button
            .map(|button| button.rect)
    }

    /// Resolve a click inside any folder-header toggle into a UI action.
    pub(crate) fn folder_header_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        [FolderPaneIdModel::Upper, FolderPaneIdModel::Lower]
            .into_iter()
            .find_map(|pane| folder_header_action(layout, model, pane, point))
    }
}

fn folder_header_layout(
    layout: &ShellLayout,
    model: &AppModel,
    pane: FolderPaneIdModel,
) -> FolderHeaderHitTargets {
    let style = style_for_layout(layout);
    let pane_model = model.sources.folder_pane(pane);
    let layout = compute_sidebar_folder_header_layout(
        sidebar_sections(layout, &style, model).folder_header(pane),
        style.sizing,
        pane_model.recovery.in_progress,
        pane_model.recovery.entry_count,
        pane_model.show_all_items,
        pane_model.can_toggle_show_all_items,
        pane_model.flattened_view,
        pane_model.can_toggle_flattened_view,
    );
    FolderHeaderHitTargets {
        visibility_toggle_button: layout.visibility_toggle_button.map(|button| {
            FolderHeaderHitButton {
                rect: button.rect,
                enabled: button.enabled,
            }
        }),
        flatten_toggle_button: layout
            .flatten_toggle_button
            .map(|button| FolderHeaderHitButton {
                rect: button.rect,
                enabled: button.enabled,
            }),
    }
}

fn folder_header_action(
    layout: &ShellLayout,
    model: &AppModel,
    pane: FolderPaneIdModel,
    point: Point,
) -> Option<UiAction> {
    let toggle = folder_header_layout(layout, model, pane);
    if let Some(button) = toggle.visibility_toggle_button
        && button.enabled
        && button.rect.contains(point)
    {
        return Some(UiAction::ToggleShowAllFolders { pane: Some(pane) });
    }
    if let Some(button) = toggle.flatten_toggle_button
        && button.enabled
        && button.rect.contains(point)
    {
        return Some(UiAction::ToggleFolderFlattenedView { pane: Some(pane) });
    }
    None
}

struct FolderHeaderHitTargets {
    visibility_toggle_button: Option<FolderHeaderHitButton>,
    flatten_toggle_button: Option<FolderHeaderHitButton>,
}

struct FolderHeaderHitButton {
    rect: Rect,
    enabled: bool,
}
