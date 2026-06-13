use crate::native_app::test_support::{
    sample_browser::{DEFAULT_FOLDER_WIDTH, MAX_FOLDER_WIDTH, MIN_FOLDER_WIDTH},
    state::NativeAppStateFixture,
};
use radiant::{gui::types::Point, widgets::DragHandleMessage};

#[test]
fn folder_browser_splitter_resizes_and_clamps_width() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build();
    state.resize_folder_browser(DragHandleMessage::started(Point::new(100.0, 0.0)));
    state.resize_folder_browser(DragHandleMessage::moved(Point::new(160.0, 0.0)));

    assert_eq!(
        state.ui.chrome.folder_panel.size(),
        DEFAULT_FOLDER_WIDTH + 60.0
    );

    state.resize_folder_browser(DragHandleMessage::moved(Point::new(900.0, 0.0)));
    assert_eq!(state.ui.chrome.folder_panel.size(), MAX_FOLDER_WIDTH);

    state.resize_folder_browser(DragHandleMessage::ended(Point::new(-900.0, 0.0)));
    assert_eq!(state.ui.chrome.folder_panel.size(), MIN_FOLDER_WIDTH);
    assert!(!state.ui.chrome.folder_panel.is_resizing());
}
