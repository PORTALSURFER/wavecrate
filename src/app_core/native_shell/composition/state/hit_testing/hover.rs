use super::*;

impl NativeShellState {
    #[cfg(test)]
    pub(crate) fn set_browser_row_hover_for_tests(&mut self, visible_row: Option<usize>) {
        self.hovered_browser_visible_row = visible_row;
    }

    /// Clear the transient browser-row hover target.
    ///
    /// Row clicks and viewport shifts can move list content underneath a
    /// stationary cursor. Clearing the row hover in those cases avoids showing
    /// an unrelated hover fill on a different row than the current selection
    /// and focus state.
    pub(crate) fn clear_browser_row_hover(&mut self) {
        self.hovered_browser_visible_row = None;
    }

    /// Synchronize the hovered folder-row overlay target during active drag/drop.
    ///
    /// Browser-row drags bypass the normal cursor-hover pipeline, so the
    /// runtime updates the folder-row hover target explicitly to keep drag
    /// feedback aligned with the pointer.
    pub(crate) fn sync_folder_drag_hover_target(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> (
        Option<(crate::compat_app_contract::FolderPaneIdModel, usize)>,
        Option<crate::compat_app_contract::FolderPaneIdModel>,
    ) {
        let hovered_folder_row = self
            .folder_row_disclosure_at_point(layout, model, point)
            .or_else(|| self.folder_row_at_point(layout, model, point));
        let over_folder_panel = self.folder_panel_at_point(layout, model, point);
        self.hovered_folder_pane = hovered_folder_row
            .map(|(pane, _)| pane)
            .or(over_folder_panel);
        self.hovered_folder_row_index = hovered_folder_row.map(|(_, row_index)| row_index);
        (hovered_folder_row, over_folder_panel)
    }

    /// Handle pointer movement and classify which overlay bucket changed.
    pub(crate) fn handle_cursor_move_effect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> CursorMoveEffect {
        let next_hover = layout.hit_test(point);
        let next_hovered_browser_row =
            self.resolve_hovered_browser_row(layout, model, point, next_hover);
        let next_hovered_browser_rating_filter_level =
            self.resolve_hovered_browser_rating_filter_level(layout, model, point);
        let next_hovered_browser_playback_age_filter_chip =
            self.resolve_hovered_browser_playback_age_filter_chip(layout, model, point);
        let next_hovered_browser_marked_filter =
            self.resolve_hovered_browser_marked_filter(layout, model, point);
        let next_hovered_browser_search_field =
            self.resolve_hovered_browser_search_field(layout, model, point);
        let next_hovered_folder_row =
            self.resolve_hovered_folder_row(layout, model, point, next_hover);
        let next_hovered_folder_pane = next_hovered_folder_row.map(|(pane, _)| pane);
        let next_hovered_source_add_button =
            self.resolve_hovered_source_add_button(layout, point, next_hover);
        let next_hovered_status_options_button =
            self.resolve_hovered_status_options_button(layout, point, next_hover);
        let next_hovered_waveform_toolbar_hint =
            self.resolve_hovered_waveform_toolbar_hint(layout, model, point, next_hover);
        let next_hovered_waveform_resize_edge =
            hovered_waveform_resize_edge_for_point(layout, model, point, next_hover);
        let next_waveform_hover_x = waveform_hover_x_for_point(layout, next_hover, point);
        let hover_changed = next_hover != self.hovered;
        let browser_row_changed = next_hovered_browser_row != self.hovered_browser_visible_row;
        let browser_rating_filter_changed =
            next_hovered_browser_rating_filter_level != self.hovered_browser_rating_filter_level;
        let browser_playback_age_filter_changed = next_hovered_browser_playback_age_filter_chip
            != self.hovered_browser_playback_age_filter_chip;
        let browser_marked_filter_changed =
            next_hovered_browser_marked_filter != self.hovered_browser_marked_filter;
        let browser_search_field_changed =
            next_hovered_browser_search_field != self.hovered_browser_search_field;
        let folder_row_changed = next_hovered_folder_row.map(|(_, row)| row)
            != self.hovered_folder_row_index
            || next_hovered_folder_pane != self.hovered_folder_pane;
        let source_add_button_changed =
            next_hovered_source_add_button != self.hovered_source_add_button;
        let status_options_button_changed =
            next_hovered_status_options_button != self.hovered_status_options_button;
        let waveform_toolbar_hint_changed =
            next_hovered_waveform_toolbar_hint != self.hovered_waveform_toolbar_hint;
        let waveform_resize_edge_changed =
            next_hovered_waveform_resize_edge != self.hovered_waveform_resize_edge;
        let waveform_hover_changed =
            next_waveform_hover_x.map(f32::to_bits) != self.waveform_hover_x.map(f32::to_bits);
        if !hover_changed
            && !browser_row_changed
            && !browser_rating_filter_changed
            && !browser_playback_age_filter_changed
            && !browser_marked_filter_changed
            && !browser_search_field_changed
            && !folder_row_changed
            && !source_add_button_changed
            && !status_options_button_changed
            && !waveform_toolbar_hint_changed
            && !waveform_resize_edge_changed
            && !waveform_hover_changed
        {
            return CursorMoveEffect::None;
        }
        self.hovered = next_hover;
        self.hovered_browser_visible_row = next_hovered_browser_row;
        self.hovered_browser_rating_filter_level = next_hovered_browser_rating_filter_level;
        self.hovered_browser_playback_age_filter_chip =
            next_hovered_browser_playback_age_filter_chip;
        self.hovered_browser_marked_filter = next_hovered_browser_marked_filter;
        self.hovered_browser_search_field = next_hovered_browser_search_field;
        self.hovered_folder_pane = next_hovered_folder_pane;
        self.hovered_folder_row_index = next_hovered_folder_row.map(|(_, row)| row);
        self.hovered_source_add_button = next_hovered_source_add_button;
        self.hovered_status_options_button = next_hovered_status_options_button;
        self.hovered_waveform_toolbar_hint = next_hovered_waveform_toolbar_hint;
        self.hovered_waveform_resize_edge = next_hovered_waveform_resize_edge;
        self.waveform_hover_x = next_waveform_hover_x;
        if waveform_hover_changed
            && !hover_changed
            && !browser_row_changed
            && !browser_rating_filter_changed
            && !browser_playback_age_filter_changed
            && !browser_marked_filter_changed
            && !browser_search_field_changed
            && !folder_row_changed
            && !source_add_button_changed
            && !status_options_button_changed
            && !waveform_toolbar_hint_changed
            && !waveform_resize_edge_changed
        {
            CursorMoveEffect::WaveformHoverOnly
        } else {
            CursorMoveEffect::GeneralOverlay
        }
    }

    fn resolve_hovered_browser_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<usize> {
        if model.map.active || hover != Some(ShellNodeKind::BrowserTable) {
            return None;
        }
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        let rows = geometry.rows;
        row_index_for_visible_rows(rows, point, layout.browser_rows)
            .map(|index| rows[index].visible_row)
    }

    fn resolve_hovered_folder_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<(crate::compat_app_contract::FolderPaneIdModel, usize)> {
        if hover != Some(ShellNodeKind::Sidebar) {
            return None;
        }
        self.folder_row_at_point(layout, model, point)
    }

    fn resolve_hovered_browser_search_field(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        toolbar.search_field.width() > 1.0 && toolbar.search_field.contains(point)
    }

    fn resolve_hovered_browser_marked_filter(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        browser_marked_filter_chip_contains_point(toolbar.marked_filter_chip, point)
    }

    fn resolve_hovered_browser_rating_filter_level(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<i8> {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        browser_rating_filter_level_at_point(toolbar.rating_filter_chips, point)
    }

    fn resolve_hovered_browser_playback_age_filter_chip(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<crate::compat_app_contract::PlaybackAgeFilterChip> {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        browser_playback_age_filter_chip_at_point(toolbar.playback_age_filter_chips, point)
    }

    fn resolve_hovered_source_add_button(
        &self,
        layout: &ShellLayout,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> bool {
        if hover != Some(ShellNodeKind::Sidebar) {
            return false;
        }
        source_add_button_rect(layout.sidebar_header, style_for_layout(layout).sizing)
            .is_some_and(|rect| rect.contains(point))
    }

    fn resolve_hovered_status_options_button(
        &self,
        layout: &ShellLayout,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> bool {
        if hover != Some(ShellNodeKind::TopBar) {
            return false;
        }
        top_bar_options_button_rect(layout.top_bar, style_for_layout(layout).sizing)
            .is_some_and(|rect| rect.contains(point))
    }

    fn resolve_hovered_waveform_toolbar_hint(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        hover: Option<ShellNodeKind>,
    ) -> Option<WaveformToolbarHoverHint> {
        if hover != Some(ShellNodeKind::WaveformCard) {
            return None;
        }
        let style = style_for_layout(layout);
        let motion_model = NativeMotionModel::from_app_model(model);
        self.cached_waveform_toolbar_buttons(layout, &style, &motion_model)
            .iter()
            .find(|button| button.rect.contains(point))
            .and_then(|button| waveform_toolbar_hover_hint(button.label))
    }
}
