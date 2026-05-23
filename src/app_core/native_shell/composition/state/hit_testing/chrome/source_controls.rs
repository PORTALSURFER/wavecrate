use super::*;
use crate::app_core::native_shell::runtime_contract::FolderPaneIdModel;

#[path = "source_controls/dropdown.rs"]
mod dropdown;
#[path = "source_controls/tags.rs"]
mod tags;

pub(in crate::app_core::native_shell::composition::state) use dropdown::sidebar_filter_dropdown_spec;
use tags::{
    sidebar_pill_editor_input_rect, sidebar_pill_editor_text_rect, sidebar_tag_action_at_point,
};

impl NativeShellState {
    /// Return the left-sidebar tag-editor input rect.
    pub(crate) fn sidebar_pill_editor_input_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let _ = model;
        sidebar_pill_editor_input_rect(layout, &style_for_layout(layout))
    }

    /// Return the left-sidebar tag-editor text rect.
    pub(crate) fn sidebar_pill_editor_text_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let _ = model;
        sidebar_pill_editor_text_rect(layout, &style_for_layout(layout))
    }

    /// Return a sidebar rating-filter chip rect in tests.
    #[cfg(test)]
    pub(crate) fn sidebar_rating_filter_chip_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        level: i8,
    ) -> Option<Rect> {
        let _ = model;
        let style = style_for_layout(layout);
        let rect = sidebar_workspace_sections(layout, &style).filters;
        let row = sidebar_filter_row_rects(rect, style.sizing)
            .get(5)
            .copied()?;
        let index = [-3, -2, -1, 0, 1, 2, 3, 4]
            .iter()
            .position(|candidate| *candidate == level)?;
        let chip = sidebar_rating_chip_rects(row, style.sizing)[index];
        (chip.width() > 1.0).then_some(chip)
    }

    /// Return a sidebar filter row rect in tests.
    #[cfg(test)]
    pub(crate) fn sidebar_filter_row_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        row_index: usize,
    ) -> Option<Rect> {
        let _ = model;
        let style = style_for_layout(layout);
        let rect = sidebar_workspace_sections(layout, &style).filters;
        sidebar_filter_row_rects(rect, style.sizing)
            .get(row_index)
            .copied()
            .filter(|row| row.width() > 1.0)
    }

    /// Resolve a rendered source-row index for a point within the sidebar.
    pub(crate) fn source_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<(FolderPaneIdModel, usize)> {
        let style = style_for_layout(layout);
        self.cached_source_rows(layout, &style, model)
            .iter()
            .find(|row| row.rect.contains(point))
            .map(|row| (row.pane, row.row_index))
    }

    /// Resolve one source context-menu action at a pointer location.
    pub(crate) fn source_context_menu_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())
    }

    /// Resolve one sidebar filter dropdown action at a pointer location.
    pub(crate) fn sidebar_filter_dropdown_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            sidebar_filter_dropdown_spec(layout, &style, model, self.sidebar_filter_dropdown)?;
        let action = buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())?;
        self.close_sidebar_filter_dropdown();
        Some(action)
    }

    /// Return `true` when a point lands inside the visible sidebar filter dropdown.
    pub(crate) fn sidebar_filter_dropdown_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        sidebar_filter_dropdown_spec(layout, &style, model, self.sidebar_filter_dropdown)
            .is_some_and(|(panel_rect, _)| panel_rect.contains(point))
    }

    /// Return a sidebar filter dropdown option rect in tests.
    #[cfg(test)]
    pub(crate) fn sidebar_filter_dropdown_option_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        option_index: usize,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        sidebar_filter_dropdown_spec(layout, &style, model, self.sidebar_filter_dropdown)
            .and_then(|(_, buttons)| buttons.get(option_index).map(|button| button.rect))
    }

    /// Return `true` when a point lands inside the visible source context menu panel.
    #[cfg(test)]
    pub(crate) fn source_context_menu_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let Some((panel_rect, _)) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)
        else {
            return false;
        };
        panel_rect.contains(point)
    }

    /// Return rendered source-row rectangles for geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_source_row_rects(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Vec<Rect> {
        self.rendered_source_row_rects_for_pane(layout, model, model.sources.active_folder_pane)
    }

    /// Return rendered source-row rectangles for one pane in geometry tests.
    #[cfg(test)]
    pub(crate) fn rendered_source_row_rects_for_pane(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pane: FolderPaneIdModel,
    ) -> Vec<Rect> {
        let style = style_for_layout(layout);
        self.cached_source_rows(layout, &style, model)
            .iter()
            .filter(|row| row.pane == pane)
            .map(|row| row.rect)
            .collect()
    }

    /// Return a source-action button rect for the provided action in tests.
    #[cfg(test)]
    pub(crate) fn source_action_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Return the sidebar-header add-source button rect in tests.
    #[cfg(test)]
    pub(crate) fn source_add_button_rect(&self, layout: &ShellLayout) -> Option<Rect> {
        source_add_button_rect(layout.sidebar_header, style_for_layout(layout).sizing)
    }

    /// Return the top-right options button rect in tests.
    #[cfg(test)]
    pub(crate) fn status_options_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        resolve_top_bar_surface_layout(
            layout.top_bar,
            style_for_layout(layout).sizing,
            &top_bar_surface_content(model),
        )
        .options_button_rect
    }

    /// Return an update-action button rect for one action in tests.
    #[cfg(test)]
    pub(crate) fn top_bar_update_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        resolve_top_bar_surface_layout(
            layout.top_bar,
            style_for_layout(layout).sizing,
            &top_bar_surface_content(model),
        )
        .update_buttons
        .into_iter()
        .find(|button| button.spec.action == action)
        .map(|button| button.rect)
    }

    /// Return whether a point falls inside the visible options panel.
    #[cfg(test)]
    pub(crate) fn options_panel_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point_with_origin(
            layout,
            &style_for_layout(layout),
            model,
            self.options_panel_origin,
            point,
        )
    }

    /// Return whether a point falls inside the visible options panel.
    pub(crate) fn options_panel_contains_point_live(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point_with_origin(
            layout,
            &style_for_layout(layout),
            model,
            self.options_panel_origin,
            point,
        )
    }

    /// Resolve a click inside the visible options panel.
    pub(crate) fn options_panel_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        options_panel_action_at_point_with_origin(
            layout,
            &style_for_layout(layout),
            model,
            self.options_panel_origin,
            point,
        )
    }

    /// Return a source-context-menu button rect for one action in tests.
    #[cfg(test)]
    pub(crate) fn source_context_menu_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            source_context_menu_spec(layout, &style, model, self.source_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Resolve a source-management action button click into a native UI action.
    pub(crate) fn source_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        if let Some(action) = sidebar_tag_action_at_point(layout, &style, model, point) {
            return Some(action);
        }
        if let Some(action) = sidebar_filter_action_at_point(self, layout, &style, model, point) {
            return Some(action);
        }
        if source_add_button_rect(layout.sidebar_header, style.sizing)
            .is_some_and(|rect| rect.contains(point))
        {
            self.trigger_source_add_button_flash();
            return Some(UiAction::OpenAddSourceDialog);
        }
        source_action_buttons(layout, &style, model)
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Resolve a sidebar background click into a section-focus action.
    pub(crate) fn sidebar_focus_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let sections = sidebar_sections(layout, &style, model);
        let pane = model.sources.active_folder_pane;
        if sections.source_rows(pane).contains(point) {
            return Some(UiAction::FocusSourcesPanel);
        }
        if sections.folder_header(pane).contains(point) || sections.tree_rows(pane).contains(point)
        {
            return Some(UiAction::FocusFolderPanel);
        }
        let workspace = sidebar_workspace_sections(layout, &style);
        if workspace.tags.contains(point) {
            return Some(UiAction::FocusBrowserPillEditorInput);
        }
        if workspace.filters.contains(point) {
            return Some(UiAction::FocusBrowserPanel);
        }
        None
    }

    /// Resolve a click inside the status-bar options button to a native options action.
    pub(crate) fn status_options_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let Some(button_rect) = resolve_top_bar_surface_layout(
            layout.top_bar,
            style_for_layout(layout).sizing,
            &top_bar_surface_content(model),
        )
        .options_button_rect
        else {
            return None;
        };
        if !button_rect.contains(point) {
            return None;
        }
        self.trigger_status_options_button_flash();
        Some(if model.options_panel.visible {
            UiAction::CloseOptionsPanel
        } else {
            UiAction::OpenOptionsMenu
        })
    }

    /// Resolve a click inside the top-bar update-action cluster.
    pub(crate) fn top_bar_update_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        resolve_top_bar_surface_layout(
            layout.top_bar,
            style_for_layout(layout).sizing,
            &top_bar_surface_content(model),
        )
        .update_buttons
        .into_iter()
        .find(|button| button.spec.enabled && button.rect.contains(point))
        .map(|button| button.spec.action)
    }
}

/// Resolve a sidebar filter-control point to its UI action.
fn sidebar_filter_action_at_point(
    shell_state: &mut NativeShellState,
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let rect = sidebar_workspace_sections(layout, style).filters;
    if rect.width() <= 1.0 || rect.height() <= 1.0 || !rect.contains(point) {
        return None;
    }
    let rows = sidebar_filter_row_rects(rect, style.sizing);
    if let Some(facet) = sidebar_filter_dropdown_facet_at_point(&rows, point) {
        shell_state.open_sidebar_filter_dropdown(facet);
        return Some(UiAction::FocusBrowserPanel);
    }
    let Some(rating_row) = rows.get(5).copied() else {
        return None;
    };
    for (index, chip) in sidebar_rating_chip_rects(rating_row, style.sizing)
        .into_iter()
        .enumerate()
    {
        if chip.contains(point) {
            return Some(UiAction::ToggleBrowserRatingFilter {
                level: [-3, -2, -1, 0, 1, 2, 3, 4][index],
                invert: false,
            });
        }
    }
    if rating_row.contains(point) {
        shell_state.open_sidebar_filter_dropdown(SidebarFilterDropdownFacet::Rating);
        return Some(UiAction::FocusBrowserPanel);
    }
    if model.browser.marked_filter_active && rows.first().is_some_and(|row| row.contains(point)) {
        return Some(UiAction::ToggleBrowserMarkedFilter);
    }
    None
}

/// Resolve non-rating sidebar filter rows to their dropdown facets.
fn sidebar_filter_dropdown_facet_at_point(
    rows: &[Rect],
    point: Point,
) -> Option<SidebarFilterDropdownFacet> {
    if rows.first().is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Format)
    } else if rows.get(1).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::BitDepth)
    } else if rows.get(2).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Channels)
    } else if rows.get(3).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Bpm)
    } else if rows.get(4).is_some_and(|row| row.contains(point)) {
        Some(SidebarFilterDropdownFacet::Key)
    } else {
        None
    }
}

/// Return local sidebar filter row rectangles.
fn sidebar_filter_row_rects(rect: Rect, sizing: SizingTokens) -> Vec<Rect> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 2.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 4.0;
    let available = (rect.height() - pad * 2.0 - title_height - gap * 5.0).max(0.0);
    let row_height = (available / 6.0)
        .min(sizing.browser_row_height.max(18.0))
        .max(8.0);
    (0..6)
        .map(|index| {
            let min_y = rect.min.y + pad + title_height + (row_height + gap) * index as f32;
            Rect::from_min_max(
                Point::new(rect.min.x + pad, min_y),
                Point::new(rect.max.x - pad, (min_y + row_height).min(rect.max.y - pad)),
            )
        })
        .collect()
}

/// Return local sidebar rating-chip hit rectangles.
fn sidebar_rating_chip_rects(rating_row: Rect, sizing: SizingTokens) -> [Rect; 8] {
    let chip_gap = 2.0_f32.max(sizing.border_width);
    let left = rating_row.min.x + (rating_row.width() * 0.43);
    let right = rating_row.max.x - sizing.text_inset_x;
    let available = (right - left - chip_gap * 7.0).max(0.0);
    let side = (available / 8.0).min(rating_row.height() - 4.0).max(0.0);
    std::array::from_fn(|index| {
        let x = left + (side + chip_gap) * index as f32;
        Rect::from_min_max(
            Point::new(x, rating_row.min.y + 2.0),
            Point::new((x + side).min(right), rating_row.min.y + 2.0 + side),
        )
    })
}
