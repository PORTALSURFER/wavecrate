use super::super::super::AppController;
use crate::app_core::actions::{NativeBrowserTagState, NativeUiAction};
use crate::app_core::state::StatusTone;

pub(super) fn apply_tagging_action(
    controller: &mut AppController,
    action: NativeUiAction,
) -> Result<(), NativeUiAction> {
    match action {
        NativeUiAction::ToggleBrowserTagSidebar => controller.toggle_browser_tag_sidebar(),
        NativeUiAction::ToggleBrowserTagSidebarAutoRename => {
            controller.toggle_browser_tag_sidebar_auto_rename()
        }
        NativeUiAction::FocusBrowserTagSidebarInput => {}
        NativeUiAction::SetBrowserTagSidebarInput { value } => {
            controller.set_browser_tag_sidebar_input(value)
        }
        NativeUiAction::CommitBrowserTagSidebarInput => {
            if let Err(err) = controller.commit_browser_tag_sidebar_input() {
                controller.set_status(format!("Could not apply tag: {err}"), StatusTone::Error);
            }
        }
        NativeUiAction::SetBrowserSidebarLooped { looped } => {
            if let Err(err) = controller.apply_browser_tag_sidebar_looped(looped) {
                controller.set_status(
                    format!("Could not update playback type: {err}"),
                    StatusTone::Error,
                );
            }
        }
        NativeUiAction::ToggleBrowserSidebarNormalTag { label } => {
            apply_normal_tag_toggle(controller, &label);
        }
        action => return Err(action),
    }
    Ok(())
}

fn apply_normal_tag_toggle(controller: &mut AppController, label: &str) {
    let result = if let Some(source) = controller.current_source() {
        let target_paths = controller.browser_tag_sidebar_target_paths();
        match controller.normal_tag_state_for_source(&source, &target_paths, label) {
            Ok(NativeBrowserTagState::On) => {
                controller.remove_browser_tag_sidebar_normal_tag(label)
            }
            Ok(_) => controller.apply_browser_tag_sidebar_normal_tag(label),
            Err(err) => Err(err),
        }
    } else {
        Err(String::from("No source selected"))
    };

    if let Err(err) = result {
        controller.set_status(format!("Could not update tag: {err}"), StatusTone::Error);
    }
}
