pub(in crate::native_app) use super::super::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::browser_context_menu;
use radiant::prelude as ui;

pub(in crate::native_app) fn browser_context_menu_overlay(
    menu: &BrowserContextMenu,
) -> ui::View<GuiMessage> {
    browser_context_menu::overlay(menu)
}
