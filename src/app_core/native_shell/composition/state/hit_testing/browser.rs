use super::super::browser_pill_editor::browser_pill_editor_layout;
use super::*;

#[path = "browser/actions.rs"]
mod actions;
#[path = "browser/cache_key.rs"]
mod cache_key;
#[path = "browser/pill_editor.rs"]
mod pill_editor;
#[path = "browser/scrollbar.rs"]
mod scrollbar;
#[cfg(test)]
#[path = "browser/test_accessors.rs"]
mod test_accessors;

pub(in crate::app_core::native_shell::composition::state) use cache_key::{
    browser_action_hit_test_cache_key, browser_action_model_signature,
};

impl NativeShellState {
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
