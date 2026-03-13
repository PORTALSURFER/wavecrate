use super::*;
use crate::ui::state::{ReleaseOption, ReleaseState, UpdateNativeBridge};
use sempal::updater::{RuntimeIdentity, UpdateChannel, UpdaterRunArgs};
use std::path::PathBuf;

fn test_args() -> UpdaterRunArgs {
    UpdaterRunArgs {
        repo: "owner/repo".to_string(),
        identity: RuntimeIdentity {
            app: "Sempal".to_string(),
            channel: UpdateChannel::Stable,
            target: "x86_64".to_string(),
            platform: "windows".to_string(),
            arch: "x86_64".to_string(),
        },
        install_dir: PathBuf::from("/tmp/sempal"),
        relaunch: true,
        requested_tag: None,
    }
}

#[test]
fn focus_action_selects_loaded_release() {
    let mut bridge = UpdateNativeBridge::new(test_args());
    bridge.release_state = ReleaseState::Loaded(vec![
        ReleaseOption {
            tag: "v1.0.0".to_string(),
            label: "v1.0.0".to_string(),
            html_url: String::new(),
        },
        ReleaseOption {
            tag: "v1.1.0".to_string(),
            label: "v1.1.0".to_string(),
            html_url: String::new(),
        },
    ]);
    bridge.on_action(UiAction::FocusBrowserRow { visible_row: 1 });
    assert_eq!(bridge.selected_tag.as_deref(), Some("v1.1.0"));
}

#[test]
fn app_model_switches_tabs_for_log_view() {
    let mut bridge = UpdateNativeBridge::new(test_args());
    bridge.on_action(UiAction::SetBrowserTab { map: true });
    let model = bridge.pull_model();
    assert_eq!(model.browser.active_tab_label.as_deref(), Some("Log"));
}
