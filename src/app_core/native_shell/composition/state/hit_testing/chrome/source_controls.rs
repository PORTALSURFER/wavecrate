use super::*;
use crate::compat_app_contract::{BrowserPillModel, BrowserPillState, FolderPaneIdModel};

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
        options_panel_contains_point(layout, &style_for_layout(layout), model, point)
    }

    /// Return whether a point falls inside the visible options panel.
    pub(crate) fn options_panel_contains_point_live(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        options_panel_contains_point(layout, &style_for_layout(layout), model, point)
    }

    /// Resolve a click inside the visible options panel.
    pub(crate) fn options_panel_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        options_panel_action_at_point(layout, &style_for_layout(layout), model, point)
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
        if let Some(action) = sidebar_filter_action_at_point(layout, &style, model, point) {
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

pub(in crate::gui::native_shell::state) fn sidebar_pill_editor_input_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> Option<Rect> {
    let rect = sidebar_workspace_sections(layout, style).tags;
    (rect.width() > 1.0 && rect.height() > 1.0).then(|| sidebar_tag_input_rect(rect, style.sizing))
}

pub(in crate::gui::native_shell::state) fn sidebar_pill_editor_text_rect(
    layout: &ShellLayout,
    style: &StyleTokens,
) -> Option<Rect> {
    sidebar_pill_editor_input_rect(layout, style)
        .map(|rect| inset_rect(rect, style.sizing.text_inset_x, style.sizing.text_inset_y))
}

fn sidebar_tag_action_at_point(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let rect = sidebar_workspace_sections(layout, style).tags;
    if rect.width() <= 1.0 || rect.height() <= 1.0 || !rect.contains(point) {
        return None;
    }
    if sidebar_tag_input_rect(rect, style.sizing).contains(point) {
        return Some(UiAction::FocusBrowserPillEditorInput);
    }
    for (pill, pill_rect) in sidebar_tag_pill_rects(rect, style.sizing, model) {
        if pill_rect.contains(point) {
            return Some(UiAction::ToggleBrowserPillOption {
                label: pill.id.clone(),
            });
        }
    }
    None
}

fn sidebar_filter_action_at_point(
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
    if model.browser.marked_filter_active && rows.get(0).is_some_and(|row| row.contains(point)) {
        return Some(UiAction::ToggleBrowserMarkedFilter);
    }
    None
}

fn sidebar_tag_input_rect(rect: Rect, sizing: SizingTokens) -> Rect {
    let pad = sizing.panel_inset.max(5.0);
    let height = sizing.browser_row_height.max(18.0);
    Rect::from_min_max(
        Point::new(
            rect.min.x + pad,
            (rect.max.y - pad - height).max(rect.min.y + pad),
        ),
        Point::new(rect.max.x - pad, rect.max.y - pad),
    )
}

fn sidebar_tag_pill_rects<'a>(
    rect: Rect,
    sizing: SizingTokens,
    model: &'a AppModel,
) -> Vec<(&'a BrowserPillModel, Rect)> {
    let pad = sizing.panel_inset.max(5.0);
    let gap = sizing.border_width.max(1.0) + 3.0;
    let title_height = sizing.font_meta + sizing.text_inset_y + 2.0;
    let input = sidebar_tag_input_rect(rect, sizing);
    let row_height = sizing.browser_row_height.max(18.0);
    let col_width = ((rect.width() - pad * 2.0 - gap) * 0.5).max(36.0);
    let mut pills: Vec<_> = model
        .browser
        .pill_editor()
        .option_pills
        .iter()
        .filter(|pill| !matches!(pill.state, BrowserPillState::Off))
        .collect();
    if pills.is_empty() {
        pills.extend(model.browser.pill_editor().option_pills.iter().take(4));
    }
    if let Some(create) = model.browser.pill_editor().create_pill.as_ref() {
        pills.push(create);
    }
    pills
        .into_iter()
        .take(4)
        .enumerate()
        .filter_map(|(index, pill)| {
            let col = index % 2;
            let row = index / 2;
            let min_x = rect.min.x + pad + (col_width + gap) * col as f32;
            let min_y = rect.min.y + pad + title_height + (row_height + gap) * row as f32;
            let pill_rect = Rect::from_min_max(
                Point::new(min_x, min_y),
                Point::new(
                    (min_x + col_width).min(rect.max.x - pad),
                    min_y + row_height,
                ),
            );
            (pill_rect.max.y <= input.min.y - gap).then_some((pill, pill_rect))
        })
        .collect()
}

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

fn inset_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::from_min_max(
        Point::new(
            (rect.min.x + x).min(rect.max.x),
            (rect.min.y + y).min(rect.max.y),
        ),
        Point::new(
            (rect.max.x - x).max(rect.min.x),
            (rect.max.y - y).max(rect.min.y),
        ),
    )
}
