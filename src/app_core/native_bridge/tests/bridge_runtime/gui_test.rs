use super::*;
use crate::gui_test::GuiTestModeConfig;
use tempfile::tempdir;

fn latest_artifact_trace_value(action: NativeUiAction) -> serde_json::Value {
    let artifact_dir = tempdir().expect("artifact tempdir");
    let mut bridge = test_bridge(16);
    bridge.install_gui_test_mode(GuiTestModeConfig {
        artifact_dir: artifact_dir.path().to_path_buf(),
        fixture_tag: String::from("bridge-runtime"),
        scenario_name: Some(String::from("bridge-runtime-trace")),
        ..GuiTestModeConfig::default()
    });
    bridge.reduce_action(action);
    let artifact_path = artifact_dir.path().join("gui_test_latest.json");
    let artifact_json =
        std::fs::read_to_string(&artifact_path).expect("live GUI artifact should exist");
    serde_json::from_str(&artifact_json).expect("artifact JSON")
}

#[test]
fn live_gui_artifact_marks_handled_actions_as_handled() {
    let artifact = latest_artifact_trace_value(NativeUiAction::FocusBrowserSearch);
    let trace = artifact["action_trace"]
        .as_array()
        .expect("action trace array");
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0]["action_id"], "focus_browser_search");
    assert_eq!(trace[0]["handled"], true);
}

#[test]
fn live_gui_artifact_marks_unhandled_actions_as_unhandled() {
    let artifact = latest_artifact_trace_value(NativeUiAction::BeginWaveformSelectionShift {
        pointer_micros: 200_000,
        start_micros: 100_000,
        end_micros: 300_000,
    });
    let trace = artifact["action_trace"]
        .as_array()
        .expect("action trace array");
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0]["action_id"], "begin_waveform_selection_shift");
    assert_eq!(trace[0]["handled"], false);
}
