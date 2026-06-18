use super::*;
use crate::app::controller::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::ui::hotkeys;
use crate::app::state::{SimilarQuery, empty_similarity_aspect_score_rows};
use crate::sample_sources::Rating;
use crate::selection::SelectionRange;
use std::path::{Path, PathBuf};

mod browser_focus;
mod similarity;
mod transport;
mod waveform_destructive;

fn action_for(
    predicate: impl Fn(&crate::app_core::actions::NativeUiAction) -> bool,
) -> HotkeyAction {
    hotkeys::find_action(predicate).expect("missing hotkey action")
}

fn sample_name(sample_id: &str) -> &str {
    sample_id
        .rsplit_once("::")
        .map_or(sample_id, |(_, sample_name)| sample_name)
}

fn seed_similarity_query_with_different_focus(controller: &mut AppController) -> (String, String) {
    controller.focus_browser_row_only(0);
    let anchor_sample_id = controller
        .sample_id_for_visible_row(0)
        .expect("anchor sample id");
    controller.ui.browser.search.similar_query = Some(SimilarQuery {
        sample_id: anchor_sample_id.clone(),
        label: String::from("kick_one.wav"),
        indices: vec![0],
        scores: vec![1.0],
        aspect_scores: empty_similarity_aspect_score_rows(1),
        anchor_index: Some(0),
    });
    controller.focus_browser_row_only(1);
    let focused_sample_id = controller
        .sample_id_for_visible_row(1)
        .expect("focused sample id");
    (anchor_sample_id, focused_sample_id)
}
