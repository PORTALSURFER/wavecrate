use super::super::super::AppController;
use crate::app_core::actions::{NativePlaybackAgeFilterChip, NativeUiAction};
use crate::app_core::state::PlaybackAgeFilterChip;

pub(super) fn apply_filter_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::ToggleBrowserRatingFilter { level, invert } => {
            controller.focus_browser_list();
            if invert {
                controller.invert_browser_rating_filter(level);
            } else {
                controller.set_browser_rating_filter(level, true);
            }
        }
        NativeUiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
            controller.focus_browser_list();
            let chip = playback_age_filter_chip(bucket);
            if invert {
                controller.invert_browser_playback_age_filter(chip);
            } else {
                controller.set_browser_playback_age_filter(chip, true);
            }
        }
        NativeUiAction::ToggleBrowserSidebarFilter { option, additive } => {
            controller.focus_browser_list();
            controller.toggle_browser_sidebar_filter(option, additive);
        }
        NativeUiAction::ClearBrowserSidebarFilter { facet } => {
            controller.focus_browser_list();
            controller.clear_browser_sidebar_filter(facet);
        }
        NativeUiAction::ToggleBrowserSampleMark => {
            controller.focus_browser_list();
            controller.toggle_browser_sample_mark();
        }
        NativeUiAction::ToggleBrowserMarkedFilter => {
            controller.focus_browser_list();
            controller.toggle_browser_marked_filter();
        }
        NativeUiAction::ToggleBrowserTagNamedFilter { invert } => {
            controller.focus_browser_list();
            controller.toggle_browser_tag_named_filter(invert);
        }
        action => return Err(action),
    }
    Ok(())
}

fn playback_age_filter_chip(bucket: NativePlaybackAgeFilterChip) -> PlaybackAgeFilterChip {
    match bucket {
        NativePlaybackAgeFilterChip::NeverPlayed => PlaybackAgeFilterChip::NeverPlayed,
        NativePlaybackAgeFilterChip::OlderThanMonth => PlaybackAgeFilterChip::OlderThanMonth,
        NativePlaybackAgeFilterChip::OlderThanWeek => PlaybackAgeFilterChip::OlderThanWeek,
    }
}
