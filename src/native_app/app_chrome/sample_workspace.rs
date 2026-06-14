use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
use crate::native_app::app_chrome::toolbar::main_toolbar;
use crate::native_app::app_chrome::view_models::sample_workspace::SampleWorkspaceViewModel;
use crate::native_app::app_chrome::waveform_panel::waveform_panel;

pub(in crate::native_app) fn region(model: SampleWorkspaceViewModel<'_>) -> ui::View<GuiMessage> {
    ui::column([
        main_toolbar(model.toolbar),
        waveform_panel(model.waveform),
        sample_browser(model.browser),
    ])
    .padding(4.0)
    .fill()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::{
        app_chrome::view_models::sample_browser::prepare_sample_browser_view,
        test_support::state::NativeAppStateFixture,
        ui::ids::{SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID, WAVEFORM_WIDGET_ID},
    };
    use radiant::layout::Vector2;
    use radiant::prelude::IntoView;

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
            .get(&crate::native_app::app_chrome::toolbar::TOOLBAR_RANDOM_ID)
            .expect("toolbar should lay out");
        let waveform = frame
            .layout
            .rects
            .get(&WAVEFORM_WIDGET_ID)
            .expect("waveform should lay out");
        let browser = frame
            .layout
            .rects
            .get(&SAMPLE_RANDOM_NAVIGATION_TOGGLE_ID)
            .expect("sample browser should lay out");

        assert!(toolbar.max.y <= waveform.min.y);
        assert!(waveform.max.y <= browser.min.y);
    }
}
