use super::rows::{
    NAME_FILTER_INPUT_ID, RATING_FILTER_SWATCH_SIZE, TAG_FILTER_INPUT_ID,
    automation_playback_type_filter_toggle_id, automation_rating_filter_toggle_id,
    empty_filter_message, name_filter_clear_button_id, rating_filter_swatch_color,
    tag_filter_clear_button_id,
};
use super::*;
use crate::native_app::sample_library::folder_browser::FolderBrowserState;
use crate::native_app::sample_library::folder_browser::model::PlaybackTypeFilter;
use radiant::prelude::IntoView;
use radiant::widgets::{ButtonMessage, SelectableMessage};

const FILTER_SECTION_TEST_FRAME_HEIGHT: f32 = 140.0;

mod layout;
mod row_projection;
