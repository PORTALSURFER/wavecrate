use crate::native_app::app::{GuiMessage, NativeAppState};
use crate::native_app::app_chrome::center_panel::center_panel;
use crate::native_app::app_chrome::settings::top_control_bar;
use crate::native_app::app_chrome::status_bar::bottom_status_area;
use radiant::prelude as ui;

pub(in crate::native_app) fn shell(state: &NativeAppState) -> ui::View<GuiMessage> {
    ui::column([
        top_control_bar(state),
        center_panel(state),
        bottom_status_area(state),
    ])
    .spacing(0.0)
    .fill()
}
