use super::*;
use crate::native_app::app::MetadataMessage;

#[test]
fn root_dispatch_routes_metadata_messages_to_metadata_owner() {
    let mut state = NativeAppState::load_default().expect("default state loads");

    state.apply_message(
        GuiMessage::Metadata(MetadataMessage::ToggleMetadataTagLibrary),
        &mut ui::UiUpdateContext::default(),
    );

    assert!(state.metadata.tag_library_open);
}
