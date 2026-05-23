use super::*;
use crate::app_core::native_shell::runtime_contract::PlaybackAgeFilterChip;

impl NativeShellState {
    /// Return a browser column-chip rect for one column index in tests.
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

    /// Return one browser rating-filter chip rect for the given signed level.
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

    /// Return the focused-row similarity button rect when present.
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
                super::super::super::browser_similarity_button_rect(row.rect, geometry.style.sizing)
            })
    }
}
