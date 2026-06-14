use super::*;

impl AppController {
    /// Tag the focused/selected wavs and keep the current focus.
    pub fn tag_selected(&mut self, target: crate::sample_sources::Rating) {
        tagging::tag_selected(self, target);
    }

    /// Move selection within the current sample browser list by an offset and play.
    pub fn nudge_selection(&mut self, offset: isize) {
        browser_nav::nudge_selection(self, offset);
    }

    /// Extend selection with shift navigation while keeping the current focus for playback.
    pub fn grow_selection(&mut self, offset: isize) {
        browser_nav::grow_selection(self, offset);
    }

    /// Cycle the triage flag filter (-1 left, +1 right) to mirror old column navigation.
    pub fn move_selection_column(&mut self, delta: isize) {
        tagging::move_selection_column(self, delta);
    }

    /// Tag leftwards: Keep -> Neutral, otherwise -> Trash.
    pub fn tag_selected_left(&mut self) {
        tagging::tag_selected_left(self);
    }

    /// Adjust rating for selected items by a delta (-3 to 3 relative change).
    pub fn adjust_selected_rating(&mut self, delta: i8) {
        tagging::adjust_selected_rating(self, delta);
    }
}
