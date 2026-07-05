use super::*;
use crate::native_app::app::MetadataMessage;
use std::time::Duration;

#[test]
fn root_dispatch_routes_metadata_messages_to_metadata_owner() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.apply_message(
        GuiMessage::Metadata(MetadataMessage::ToggleMetadataTagLibrary),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
}

#[test]
fn frame_messages_use_frame_budget_slow_threshold() {
    assert_eq!(
        slow_ui_message_threshold(FRAME_MESSAGE_PROFILE_LABEL),
        Duration::from_micros(16_667)
    );
    assert_eq!(
        slow_ui_message_threshold("NavigateBrowser"),
        Duration::from_millis(4)
    );
}
