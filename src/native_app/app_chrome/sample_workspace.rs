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
