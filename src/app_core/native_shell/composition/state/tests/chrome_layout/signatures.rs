use super::super::*;

#[test]
fn browser_action_model_signature_changes_with_action_flags_and_chip_content() {
    let mut baseline = AppModel::default();
    baseline.browser_actions.can_rename = true;
    baseline.browser_actions.can_edit_pills = true;
    baseline.browser_actions.can_delete = false;
    baseline.columns[0].title = String::from("Trash");
    baseline.columns[0].item_count = 10;
    baseline.columns[1].title = String::from("Neutral");
    baseline.columns[1].item_count = 20;
    baseline.columns[2].title = String::from("Keep");
    baseline.columns[2].item_count = 30;

    let baseline_signature = browser_action_model_signature(&baseline);

    let mut changed_flag = baseline.clone();
    changed_flag.browser_actions.can_delete = true;
    assert_ne!(
        baseline_signature,
        browser_action_model_signature(&changed_flag)
    );

    let mut changed_chip = baseline.clone();
    changed_chip.columns[2].title = String::from("Favorites");
    assert_ne!(
        baseline_signature,
        browser_action_model_signature(&changed_chip)
    );
}

#[test]
fn waveform_toolbar_model_flags_change_with_channel_and_toggle_state() {
    let baseline = NativeMotionModel::from_app_model(&AppModel::default());
    let baseline_flags = waveform_toolbar_model_flags(&baseline);

    let mut changed_channel = baseline.clone();
    changed_channel.waveform_channel_view = match baseline.waveform_channel_view {
        crate::compat_app_contract::WaveformChannelViewModel::Mono => {
            crate::compat_app_contract::WaveformChannelViewModel::Stereo
        }
        crate::compat_app_contract::WaveformChannelViewModel::Stereo => {
            crate::compat_app_contract::WaveformChannelViewModel::Mono
        }
    };
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_channel)
    );

    let mut changed_toggle = baseline.clone();
    changed_toggle.waveform_bpm_snap_enabled = !baseline.waveform_bpm_snap_enabled;
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_toggle)
    );

    let mut changed_relative_grid = baseline.clone();
    changed_relative_grid.waveform_relative_bpm_grid_enabled =
        !baseline.waveform_relative_bpm_grid_enabled;
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_relative_grid)
    );

    let mut changed_compare_anchor = baseline.clone();
    changed_compare_anchor.waveform_compare_anchor_available =
        !baseline.waveform_compare_anchor_available;
    assert_ne!(
        baseline_flags,
        waveform_toolbar_model_flags(&changed_compare_anchor)
    );
}
