use radiant::{prelude as ui, widgets::DragHandleMessage};

use crate::native_app::app::GuiMessage;
use crate::native_app::sample_library::folder_browser::view_contract::SIDEBAR_PANEL_HEADER_HEIGHT;

pub(super) fn sidebar_resize_header(
    key: impl ToString,
    widget_id: u64,
    map: impl Fn(DragHandleMessage) -> GuiMessage + Send + Sync + 'static,
) -> ui::View<GuiMessage> {
    ui::panel_section_resize_header(key, SIDEBAR_PANEL_HEADER_HEIGHT, map).id(widget_id)
}
