use super::*;
use crate::gui::list::{
    VirtualListScrollbar, virtual_list_scrollbar_thumb_offset_at_point,
    virtual_list_scrollbar_view_start_at_point,
};

/// Additional hit slop for the narrow content-list scrollbar thumb.
const BROWSER_SCROLLBAR_THUMB_HIT_SLOP: f32 = 3.0;

impl NativeShellState {
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
}
