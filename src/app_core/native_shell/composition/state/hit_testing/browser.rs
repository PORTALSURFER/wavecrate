#[cfg(test)]
use self::sempal_crate::app as native_model;
use super::*;
#[cfg(test)]
use crate as sempal_crate;

impl NativeShellState {
    /// Return a browser column-chip rect for one column index in tests.
    #[cfg(test)]
    pub(crate) fn browser_column_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        column: usize,
    ) -> Option<Rect> {
        self.cached_browser_interaction_geometry(layout, model)
            .chips
            .iter()
            .find(|chip| chip.column == column)
            .map(|chip| chip.rect)
    }

    /// Resolve a rendered browser visible-row index for a point in the triage pane.
    pub(crate) fn browser_row_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        if model.map.active {
            return None;
        }
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        if let Some(sidebar_rect) =
            browser_pill_editor_panel_rect(layout.browser_rows, geometry.style.sizing, model)
            && sidebar_rect.contains(point)
        {
            return None;
        }
        let list_rect = browser_rows_list_rect(layout.browser_rows, geometry.style.sizing, model);
        let rows = geometry.rows;
        row_index_for_visible_rows(rows, point, list_rect).map(|index| rows[index].visible_row)
    }

    /// Resolve the focused-row similarity button into its native action.
    pub(crate) fn browser_row_similarity_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if model.map.active || model.browser.duplicate_cleanup_active {
            return None;
        }
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        geometry
            .rows
            .iter()
            .find(|row| row.focused)
            .and_then(|row| browser_similarity_button_rect(row.rect, geometry.style.sizing))
            .filter(|rect| rect.contains(point))
            .map(|_| focused_similarity_action())
    }

    /// Resolve one browser context-menu action at a pointer location.
    pub(crate) fn browser_context_menu_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            browser_context_menu_spec(layout, &style, model, self.browser_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action)
    }

    /// Return `true` when a point lands inside the visible browser context menu panel.
    #[cfg(test)]
    pub(crate) fn browser_context_menu_contains_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> bool {
        let style = style_for_layout(layout);
        let Some((panel_rect, _)) =
            browser_context_menu_spec(layout, &style, model, self.browser_context_menu)
        else {
            return false;
        };
        panel_rect.contains(point)
    }

    /// Return a browser-context-menu button rect for one action in tests.
    #[cfg(test)]
    pub(crate) fn browser_context_menu_button_rect(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        action: UiAction,
    ) -> Option<Rect> {
        let style = style_for_layout(layout);
        let (_, buttons) =
            browser_context_menu_spec(layout, &style, model, self.browser_context_menu)?;
        buttons
            .into_iter()
            .find(|button| button.action == action)
            .map(|button| button.rect)
    }

    /// Return the current rendered browser viewport length.
    pub(crate) fn browser_viewport_len(&mut self, layout: &ShellLayout, model: &AppModel) -> usize {
        self.cached_browser_interaction_geometry(layout, model)
            .rows
            .len()
            .min(model.browser.visible_count)
    }

    /// Return the current rendered browser viewport start row.
    ///
    /// The shell can preserve a previously resolved visible window even when the
    /// host-projected `view_start_row` is briefly stale. Callers that need to
    /// continue scrolling from the rows the user is actually seeing should use
    /// this value instead of the raw model field.
    pub(crate) fn browser_viewport_start_row(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<usize> {
        self.cached_browser_interaction_geometry(layout, model)
            .rows
            .first()
            .map(|row| row.visible_row)
    }

    /// Return the pointer's offset within the browser scrollbar thumb when hovered.
    pub(crate) fn browser_scrollbar_thumb_offset_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<f32> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        let scrollbar = geometry.scrollbar?;
        let hit_rect = Rect::from_min_max(
            Point::new(
                scrollbar.track.min.x - BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
                scrollbar.thumb.min.y - BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
            ),
            Point::new(
                scrollbar.track.max.x + BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
                scrollbar.thumb.max.y + BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
            ),
        );
        hit_rect
            .contains(point)
            .then_some((point.y - scrollbar.thumb.min.y).clamp(0.0, scrollbar.thumb.height()))
    }

    /// Resolve the browser viewport start row for an active scrollbar-thumb drag.
    pub(crate) fn browser_scrollbar_view_start_for_drag(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        pointer_y: f32,
        thumb_pointer_offset_y: f32,
    ) -> Option<usize> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        let scrollbar = geometry.scrollbar?;
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            geometry.scrollbar_viewport_len,
            model.browser.visible_count,
            pointer_y,
            thumb_pointer_offset_y,
        )
    }

    /// Resolve the browser viewport start for a click inside the scrollbar track.
    ///
    /// Track clicks jump the thumb so its center aligns with the clicked
    /// location, matching the visual expectation that the handle should move to
    /// the requested position immediately.
    pub(crate) fn browser_scrollbar_view_start_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<usize> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        let scrollbar = geometry.scrollbar?;
        if !scrollbar.track.contains(point) || scrollbar.thumb.contains(point) {
            return None;
        }
        browser_scrollbar_view_start_for_pointer(
            scrollbar,
            geometry.scrollbar_viewport_len,
            model.browser.visible_count,
            point.y,
            scrollbar.thumb.height() * 0.5,
        )
    }

    /// Resolve a browser action-strip click into a native UI action.
    pub(crate) fn browser_action_at_point(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
        alt_down: bool,
    ) -> Option<UiAction> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        if let Some(action) = browser_pill_editor_action_at_point(
            layout.browser_rows,
            geometry.style.sizing,
            model,
            point,
        ) {
            return Some(action);
        }
        if let Some(level) =
            browser_rating_filter_level_at_point(geometry.toolbar.rating_filter_chips, point)
        {
            return Some(UiAction::ToggleBrowserRatingFilter {
                level,
                invert: alt_down,
            });
        }
        if let Some(bucket) = browser_playback_age_filter_chip_at_point(
            geometry.toolbar.playback_age_filter_chips,
            point,
        ) {
            return Some(UiAction::ToggleBrowserPlaybackAgeFilter {
                bucket,
                invert: alt_down,
            });
        }
        if browser_marked_filter_chip_contains_point(geometry.toolbar.marked_filter_chip, point) {
            return Some(UiAction::ToggleBrowserMarkedFilter);
        }
        if browser_marked_filter_chip_contains_point(
            geometry.toolbar.derived_label_filter_chip,
            point,
        ) {
            return Some(UiAction::ToggleBrowserDerivedLabelFilter { invert: alt_down });
        }
        if geometry.toolbar.search_field.width() > 1.0
            && geometry.toolbar.search_field.contains(point)
        {
            return Some(UiAction::FocusBrowserSearch);
        }
        if let Some(action) = geometry
            .chips
            .iter()
            .find(|chip| chip.rect.contains(point))
            .map(|chip| UiAction::SelectColumn { index: chip.column })
        {
            return Some(action);
        }
        geometry
            .buttons
            .iter()
            .find(|button| button.enabled && button.rect.contains(point))
            .map(|button| button.action.clone())
    }

    /// Resolve a browser tab click into a list/map tab selection action.
    pub(crate) fn browser_tab_action_at_point(
        &self,
        layout: &ShellLayout,
        point: Point,
    ) -> Option<UiAction> {
        let tabs: BrowserTabsRects = {
            let style = style_for_layout(layout);
            let tabs = resolve_browser_tabs_surface_layout(
                layout.browser_tabs,
                style.sizing,
                &BrowserTabsSurfaceContent {
                    items_label: String::new(),
                    map_label: String::new(),
                },
            );
            BrowserTabsRects {
                items: tabs.items,
                map: tabs.map,
            }
        };
        if tabs.items.contains(point) {
            return Some(UiAction::SetBrowserTab { map: false });
        }
        if tabs.map.contains(point) {
            return Some(UiAction::SetBrowserTab { map: true });
        }
        None
    }

    /// Return the browser-search field rect when the toolbar is available.
    pub(crate) fn browser_search_field_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        (geometry.toolbar.search_field.width() > 1.0).then_some(geometry.toolbar.search_field)
    }

    /// Return one browser rating-filter chip rect for the given signed level.
    #[cfg(test)]
    pub(crate) fn browser_rating_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        level: i8,
    ) -> Option<Rect> {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        let index = browser_rating_filter_chip_index(level)?;
        let rect = toolbar.rating_filter_chips[index];
        (rect.width() > 1.0).then_some(rect)
    }

    /// Return the marked-filter chip rect when the toolbar is available.
    #[cfg(test)]
    pub(crate) fn browser_marked_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        (toolbar.marked_filter_chip.width() > 1.0).then_some(toolbar.marked_filter_chip)
    }

    /// Return the derived-label-filter chip rect when the toolbar is available.
    #[cfg(test)]
    pub(crate) fn browser_derived_label_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        (toolbar.derived_label_filter_chip.width() > 1.0)
            .then_some(toolbar.derived_label_filter_chip)
    }

    /// Return one browser playback-age filter chip rect for the given chip.
    #[cfg(test)]
    pub(crate) fn browser_playback_age_filter_chip_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        chip: native_model::PlaybackAgeFilterChip,
    ) -> Option<Rect> {
        let toolbar = self
            .cached_browser_interaction_geometry(layout, model)
            .toolbar;
        let index = browser_playback_age_filter_chip_index(chip)?;
        let rect = toolbar.playback_age_filter_chips[index];
        (rect.width() > 1.0).then_some(rect)
    }

    /// Return one browser action-button rect for the given label.
    #[cfg(test)]
    pub(crate) fn browser_action_button_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
        label: &str,
    ) -> Option<Rect> {
        self.cached_browser_interaction_geometry(layout, model)
            .buttons
            .iter()
            .find(|button| button.label == label)
            .map(|button| button.rect)
    }

    /// Return the browser-search text rect used for rendering inside the field.
    pub(crate) fn browser_search_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        if geometry.toolbar.search_field.width() <= 1.0 {
            return None;
        }
        let toolbar_text_layout = compute_browser_toolbar_text_layout(
            geometry.toolbar.search_field,
            geometry.toolbar.activity_chip,
            geometry.toolbar.sort_chip,
            geometry.style.sizing,
        );
        Some(toolbar_text_layout.search_label)
    }

    /// Return the browser tag-sidebar input rect when the sidebar is visible.
    pub(crate) fn browser_pill_editor_input_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = self
            .cached_browser_interaction_geometry(layout, model)
            .style;
        browser_pill_editor_layout(layout.browser_rows, style.sizing, model)
            .map(|layout| layout.input_rect)
    }

    /// Return the browser tag-sidebar input text rect when the sidebar is visible.
    pub(crate) fn browser_pill_editor_text_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let style = self
            .cached_browser_interaction_geometry(layout, model)
            .style;
        browser_pill_editor_layout(layout.browser_rows, style.sizing, model)
            .map(|layout| layout.input_text_rect)
    }

    /// Return the focused-row similarity button rect when present.
    #[cfg(test)]
    pub(crate) fn browser_similarity_button_rect(
        &mut self,
        layout: &ShellLayout,
        model: &AppModel,
    ) -> Option<Rect> {
        let geometry = self.cached_browser_interaction_geometry(layout, model);
        geometry
            .rows
            .iter()
            .find(|row| row.focused)
            .and_then(|row| {
                super::super::browser_similarity_button_rect(row.rect, geometry.style.sizing)
            })
    }

    /// Resolve a map-point click to a focus action when map tab is active.
    pub(crate) fn map_content_action_at_point(
        &self,
        layout: &ShellLayout,
        model: &AppModel,
        point: Point,
    ) -> Option<UiAction> {
        if !model.map.active {
            return None;
        }
        map_content_id_at_point(layout, model, point).map(map_focus_action)
    }
}

fn map_focus_action(content_id: String) -> UiAction {
    #[cfg(feature = "legacy-shell")]
    {
        UiAction::FocusSpatialContentItem { content_id }
    }
    #[cfg(not(feature = "legacy-shell"))]
    {
        let sample_id = content_id;
        UiAction::FocusMapSample { sample_id }
    }
}

fn focused_similarity_action() -> UiAction {
    #[cfg(feature = "legacy-shell")]
    {
        UiAction::ToggleFindSimilarFocusedContent
    }
    #[cfg(not(feature = "legacy-shell"))]
    {
        UiAction::ToggleFindSimilarFocusedSample
    }
}

#[derive(Clone, Debug)]
struct BrowserPillEditorLayout {
    auto_rename_rect: Rect,
    input_rect: Rect,
    input_text_rect: Rect,
    playback_rects: [Rect; 2],
    normal_tag_rects: Vec<Rect>,
    create_tag_rect: Option<Rect>,
}

fn browser_pill_editor_rect(
    rows_rect: Rect,
    _sizing: SizingTokens,
    model: &AppModel,
) -> Option<Rect> {
    browser_pill_editor_panel_rect(rows_rect, _sizing, model)
}

fn browser_pill_editor_layout(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
) -> Option<BrowserPillEditorLayout> {
    let rect = browser_pill_editor_rect(rows_rect, sizing, model)?;
    let pad = sizing.panel_inset.max(8.0);
    let content_min_x = rect.min.x + pad;
    let content_max_x = rect.max.x - pad;
    let field_height = sizing.browser_row_height.max(22.0);
    let auto_rename_top = rect.min.y + pad + sizing.font_body + 10.0;
    let auto_rename_rect = Rect::from_min_max(
        Point::new(content_min_x, auto_rename_top),
        Point::new(content_max_x, auto_rename_top + field_height),
    );
    let input_top = auto_rename_rect.max.y + 8.0;
    let input_rect = Rect::from_min_max(
        Point::new(content_min_x, input_top),
        Point::new(content_max_x, input_top + field_height),
    );
    let input_text_rect = Rect::from_min_max(
        Point::new(
            input_rect.min.x + sizing.text_inset_x,
            input_rect.min.y + sizing.text_inset_y,
        ),
        Point::new(
            input_rect.max.x - sizing.text_inset_x,
            input_rect.max.y - sizing.text_inset_y,
        ),
    );
    let pill_gap = sizing.border_width.max(1.0) + 4.0;
    let two_col_width = ((content_max_x - content_min_x - pill_gap) * 0.5).max(40.0);
    let playback_top = input_rect.max.y + 10.0;
    let playback_rects = [
        Rect::from_min_max(
            Point::new(content_min_x, playback_top),
            Point::new(content_min_x + two_col_width, playback_top + field_height),
        ),
        Rect::from_min_max(
            Point::new(content_min_x + two_col_width + pill_gap, playback_top),
            Point::new(content_max_x, playback_top + field_height),
        ),
    ];
    let tags_top = playback_rects[0].max.y + 12.0;
    let tag_cols = 3usize;
    let tag_width = ((content_max_x - content_min_x - pill_gap * (tag_cols - 1) as f32)
        / tag_cols as f32)
        .max(40.0);
    let mut normal_tag_rects = Vec::with_capacity(model.browser.pill_editor().option_pills.len());
    for index in 0..model.browser.pill_editor().option_pills.len() {
        let col = index % tag_cols;
        let row = index / tag_cols;
        let min_x = content_min_x + (tag_width + pill_gap) * col as f32;
        let min_y = tags_top + (field_height + pill_gap) * row as f32;
        normal_tag_rects.push(Rect::from_min_max(
            Point::new(min_x, min_y),
            Point::new((min_x + tag_width).min(content_max_x), min_y + field_height),
        ));
    }
    let create_tag_rect = model.browser.pill_editor().create_pill.as_ref().map(|_| {
        let y = normal_tag_rects
            .last()
            .map(|rect| rect.max.y + 12.0)
            .unwrap_or(tags_top);
        Rect::from_min_max(
            Point::new(content_min_x, y),
            Point::new(content_max_x, y + field_height),
        )
    });
    Some(BrowserPillEditorLayout {
        auto_rename_rect,
        input_rect,
        input_text_rect,
        playback_rects,
        normal_tag_rects,
        create_tag_rect,
    })
}

fn browser_pill_editor_action_at_point(
    rows_rect: Rect,
    sizing: SizingTokens,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let layout = browser_pill_editor_layout(rows_rect, sizing, model)?;
    if layout.auto_rename_rect.contains(point) {
        return Some(UiAction::ToggleBrowserPillEditorPrimaryAction);
    }
    if layout.input_rect.contains(point) {
        return Some(UiAction::FocusBrowserPillEditorInput);
    }
    for (index, rect) in layout.playback_rects.iter().enumerate() {
        if rect.contains(point) {
            return Some(UiAction::SetBrowserSidebarLooped { looped: index == 0 });
        }
    }
    for (pill, rect) in model
        .browser
        .pill_editor()
        .option_pills
        .iter()
        .zip(layout.normal_tag_rects.iter())
    {
        if rect.contains(point) {
            return Some(UiAction::ToggleBrowserPillOption {
                label: pill.label.clone(),
            });
        }
    }
    if let (Some(pill), Some(rect)) = (
        model.browser.pill_editor().create_pill.as_ref(),
        layout.create_tag_rect,
    ) && rect.contains(point)
    {
        return Some(UiAction::ToggleBrowserPillOption {
            label: pill.id.clone(),
        });
    }
    None
}

pub(in crate::gui::native_shell::state) fn browser_action_hit_test_cache_key(
    layout: &ShellLayout,
    model: &AppModel,
) -> BrowserActionHitTestCacheKey {
    BrowserActionHitTestCacheKey {
        browser_toolbar_min_x: f32_to_bits(layout.browser_toolbar.min.x),
        browser_toolbar_min_y: f32_to_bits(layout.browser_toolbar.min.y),
        browser_toolbar_max_x: f32_to_bits(layout.browser_toolbar.max.x),
        browser_toolbar_max_y: f32_to_bits(layout.browser_toolbar.max.y),
        ui_scale: f32_to_bits(layout.ui_scale),
        model_signature: browser_action_model_signature(model),
    }
}

pub(in crate::gui::native_shell::state) fn browser_action_model_signature(model: &AppModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    model.browser_actions.can_rename.hash(&mut hasher);
    model.browser_actions.can_edit_pills().hash(&mut hasher);
    model.browser_actions.can_delete.hash(&mut hasher);
    model
        .browser_actions
        .random_navigation_enabled
        .hash(&mut hasher);
    model
        .browser_actions
        .duplicate_cleanup_active
        .hash(&mut hasher);
    model.browser_actions.pill_editor_open().hash(&mut hasher);
    model.browser.active_rating_filters.hash(&mut hasher);
    model.browser.active_recency_filters.hash(&mut hasher);
    model.browser.marked_filter_active.hash(&mut hasher);
    model
        .browser
        .derived_label_filter_active()
        .hash(&mut hasher);
    model
        .browser
        .derived_label_filter_negated()
        .hash(&mut hasher);
    model.browser.search_query.hash(&mut hasher);
    model.browser.busy.hash(&mut hasher);
    model.browser.sort_label.hash(&mut hasher);
    model.browser_chrome.search_placeholder.hash(&mut hasher);
    model.browser_chrome.activity_ready_label.hash(&mut hasher);
    model.browser_chrome.activity_busy_label.hash(&mut hasher);
    model.browser_chrome.sort_prefix_label.hash(&mut hasher);
    model.browser_chrome.sort_order_label.hash(&mut hasher);
    model.selected_column.min(2).hash(&mut hasher);
    for index in 0..3 {
        if let Some(column) = model.columns.get(index) {
            column.title.hash(&mut hasher);
            column.item_count.hash(&mut hasher);
        } else {
            index.hash(&mut hasher);
        }
    }
    hasher.finish()
}
