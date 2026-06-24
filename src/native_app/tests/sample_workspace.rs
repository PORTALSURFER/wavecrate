use crate::native_app::{
    app_chrome::{
        sample_workspace::region,
        toolbar::TOOLBAR_RANDOM_ID,
        view_models::{
            sample_browser::prepare_sample_browser_view, sample_workspace::SampleWorkspaceViewModel,
        },
    },
    test_support::state::NativeAppStateFixture,
    ui::ids::{AUTOMATION_SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID, WAVEFORM_WIDGET_ID},
};
use radiant::{layout::Vector2, prelude::IntoView};

#[test]
fn sample_workspace_projects_toolbar_waveform_and_browser_in_order() {
    let mut state = NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .build();
    prepare_sample_browser_view(&mut state);

    let frame = region(SampleWorkspaceViewModel::from_app_state(&state))
        .view_frame_at_size_with_default_theme(Vector2::new(900.0, 620.0));
    let toolbar = frame
        .layout
        .rects
        .get(&TOOLBAR_RANDOM_ID)
        .expect("toolbar should lay out");
    let waveform = frame
        .layout
        .rects
        .get(&WAVEFORM_WIDGET_ID)
        .expect("waveform should lay out");
    let browser = frame
        .layout
        .rects
        .get(&AUTOMATION_SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID)
        .expect("sample browser should lay out");

    assert!(toolbar.max.y <= waveform.min.y);
    assert!(waveform.max.y <= browser.min.y);
}
