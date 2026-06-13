use crate::native_app::test_support::state::WaveformState;

#[test]
fn default_gui_starts_without_loading_a_sample() {
    let waveform = WaveformState::load_default().expect("default sample loads");
    assert!(!waveform.has_loaded_sample());
    assert_eq!(waveform.file_name(), "No sample loaded");
}
