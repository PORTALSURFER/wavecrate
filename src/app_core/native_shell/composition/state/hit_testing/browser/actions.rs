use super::pill_editor::browser_pill_editor_action_at_point;
use super::*;

impl NativeShellState {
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
        if let Some(action) = browser_toolbar_action_at_point(&geometry.toolbar, point, alt_down) {
            return Some(action);
        }
        if let Some(action) = browser_column_action_at_point(geometry.chips, point) {
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
        let tabs = browser_tabs_rects(layout);
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
}

fn browser_toolbar_action_at_point(
    toolbar: &BrowserToolbarLayout,
    point: Point,
    alt_down: bool,
) -> Option<UiAction> {
    if let Some(level) = browser_rating_filter_level_at_point(toolbar.rating_filter_chips, point) {
        return Some(UiAction::ToggleBrowserRatingFilter {
            level,
            invert: alt_down,
        });
    }
    if let Some(bucket) =
        browser_playback_age_filter_chip_at_point(toolbar.playback_age_filter_chips, point)
    {
        return Some(UiAction::ToggleBrowserPlaybackAgeFilter {
            bucket,
            invert: alt_down,
        });
    }
    if browser_marked_filter_chip_contains_point(toolbar.marked_filter_chip, point) {
        return Some(UiAction::ToggleBrowserMarkedFilter);
    }
    if browser_marked_filter_chip_contains_point(toolbar.derived_label_filter_chip, point) {
        return Some(UiAction::ToggleBrowserDerivedLabelFilter { invert: alt_down });
    }
    (toolbar.search_field.width() > 1.0 && toolbar.search_field.contains(point))
        .then_some(UiAction::FocusBrowserSearch)
}

fn browser_column_action_at_point(chips: &[BrowserColumnChip], point: Point) -> Option<UiAction> {
    chips
        .iter()
        .find(|chip| chip.rect.contains(point))
        .map(|chip| UiAction::SelectColumn { index: chip.column })
}

fn browser_tabs_rects(layout: &ShellLayout) -> BrowserTabsRects {
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
}
