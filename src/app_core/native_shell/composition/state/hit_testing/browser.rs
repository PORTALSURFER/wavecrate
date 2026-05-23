use super::super::browser_pill_editor::browser_pill_editor_layout;
use super::*;
#[cfg(test)]
use crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip;
use crate::gui::list::{
    VirtualListScrollbar, virtual_list_scrollbar_thumb_offset_at_point,
    virtual_list_scrollbar_view_start_at_point,
};

#[path = "browser/cache_key.rs"]
mod cache_key;
#[path = "browser/pill_editor.rs"]
mod pill_editor;

pub(in crate::app_core::native_shell::composition::state) use cache_key::{
    browser_action_hit_test_cache_key, browser_action_model_signature,
};
use pill_editor::browser_pill_editor_action_at_point;

/// Additional hit slop for the narrow content-list scrollbar thumb.
const BROWSER_SCROLLBAR_THUMB_HIT_SLOP: f32 = 3.0;

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
        virtual_list_scrollbar_thumb_offset_at_point(
            VirtualListScrollbar {
                track: scrollbar.track,
                thumb: scrollbar.thumb,
            },
            point,
            BROWSER_SCROLLBAR_THUMB_HIT_SLOP,
        )
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
        virtual_list_scrollbar_view_start_at_point(
            VirtualListScrollbar {
                track: scrollbar.track,
                thumb: scrollbar.thumb,
            },
            geometry.scrollbar_viewport_len,
            model.browser.visible_count,
            point,
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
        chip: PlaybackAgeFilterChip,
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
            .or_else(|| self.sidebar_pill_editor_input_rect(layout, model))
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
            .or_else(|| self.sidebar_pill_editor_text_rect(layout, model))
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
    // The current local composition contract is generic. The outer Wavecrate
    // adapter still maps this to UiAction::FocusMapSample for product code.
    UiAction::FocusSpatialContentItem { content_id }
}

fn focused_similarity_action() -> UiAction {
    // The current local composition contract is generic. The outer Wavecrate
    // adapter still maps this to UiAction::ToggleFindSimilarFocusedSample.
    UiAction::ToggleFindSimilarFocusedContent
}
