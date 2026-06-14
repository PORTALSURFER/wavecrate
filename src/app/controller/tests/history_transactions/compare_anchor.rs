use crate::app::controller::state::selection::CompareAnchorSample;
use crate::app::controller::test_support::{prepare_with_source_and_wav_entries, sample_entry};
use crate::app::state::CompareAnchorState;
use std::path::PathBuf;

#[test]
fn meaningful_ui_undo_keeps_compare_anchor_transient_state() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("anchor.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("current.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.set_compare_anchor_from_focused_browser_sample();
    let expected_sample = CompareAnchorSample {
        source_id: source.id.clone(),
        relative_path: PathBuf::from("anchor.wav"),
    };
    let expected_ui = CompareAnchorState {
        source_id: source.id,
        relative_path: PathBuf::from("anchor.wav"),
        label: String::from("anchor"),
    };

    controller.focus_browser_row_only(1);
    controller.toggle_browser_row_selection(1);

    controller.undo();
    assert_eq!(
        controller.sample_view.wav.compare_anchor,
        Some(expected_sample.clone())
    );
    assert_eq!(controller.ui.compare_anchor, Some(expected_ui.clone()));
    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("anchor")
    );

    controller.redo();
    assert_eq!(
        controller.sample_view.wav.compare_anchor,
        Some(expected_sample)
    );
    assert_eq!(controller.ui.compare_anchor, Some(expected_ui));
    assert_eq!(
        controller.ui.waveform.compare_anchor_label.as_deref(),
        Some("anchor")
    );
}
