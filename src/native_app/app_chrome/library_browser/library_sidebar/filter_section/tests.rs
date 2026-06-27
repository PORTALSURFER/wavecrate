use super::rows::{
    CURATION_FILTER_DROPDOWN_TRIGGER_ID, HARVEST_FILTER_DROPDOWN_TRIGGER_ID, NAME_FILTER_INPUT_ID,
    RATING_FILTER_SWATCH_SIZE, TAG_FILTER_INPUT_ID, automation_curation_filter_dropdown_option_id,
    automation_filter_family_label_toggle_id, automation_harvest_filter_dropdown_option_id,
    automation_playback_type_filter_toggle_id, automation_rating_filter_toggle_id,
    empty_filter_message, name_filter_clear_button_id, rating_filter_swatch_color,
    tag_filter_clear_button_id,
};
use super::*;
use crate::native_app::sample_library::folder_browser::FolderBrowserState;
use crate::native_app::sample_library::folder_browser::commands::FilterFamily;
use crate::native_app::sample_library::folder_browser::model::{
    BrowserCurationScope, HarvestFilter, PlaybackTypeFilter,
};
use crate::native_app::ui::ids::HARVEST_FAMILY_TOGGLE_ID;
use radiant::gui::types::Rgba8;
use radiant::prelude::IntoView;
use radiant::runtime::{SurfaceFrame, SurfacePaintPlan};
use radiant::widgets::{ButtonMessage, SelectableMessage, TextInputMessage};

const FILTER_SECTION_TEST_FRAME_HEIGHT: f32 = 220.0;

mod layout;
mod row_projection;
