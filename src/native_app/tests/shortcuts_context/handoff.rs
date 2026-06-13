use crate::native_app::test_support::{
    state::{GuiMessage, NativeAppState, default_gui_shortcuts},
    waveform::format_copy_path,
};
use radiant::prelude as ui;

#[test]
fn format_copy_path_uses_forward_slashes_and_quotes_spaces() {
    assert_eq!(
        format_copy_path(std::path::Path::new("C:\\sample folder\\kick.wav")),
        "\"C:/sample folder/kick.wav\""
    );
    assert_eq!(
        format_copy_path(std::path::Path::new("C:\\samples\\kick.wav")),
        "C:/samples/kick.wav"
    );
}

#[test]
fn copy_shortcut_routes_to_browser_file_handoff() {
    let state = NativeAppState::load_default().expect("default state loads");
    let resolution =
        default_gui_shortcuts(&state).resolve(ui::KeyPress::with_command(ui::KeyCode::C));

    assert_eq!(resolution.action, Some(GuiMessage::CopySelectedFiles));
    assert!(resolution.handled);
}
