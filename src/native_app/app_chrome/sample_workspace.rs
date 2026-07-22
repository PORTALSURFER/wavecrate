use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::sample_browser_view::sample_browser;
use crate::native_app::app_chrome::view_models::sample_workspace::SampleWorkspaceViewModel;
use crate::native_app::app_chrome::waveform_panel::waveform_panel;

pub(in crate::native_app) fn region(model: SampleWorkspaceViewModel<'_>) -> ui::View<GuiMessage> {
    ui::column([
        waveform_panel(model.waveform),
        sample_browser(model.browser),
    ])
    .spacing(0.0)
    .fill()
}
