pub(in crate::native_app) use super::super::sample_library::context_menu_target::{
    BrowserContextMenu, BrowserContextTargetKind,
};
pub(in crate::native_app) use super::super::waveform::WaveformContextMenu;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::browser_context_menu;
use crate::native_app::app_chrome::waveform_context_menu;
use radiant::prelude as ui;

pub(in crate::native_app) fn browser_context_menu_overlay(
    menu: &BrowserContextMenu,
) -> ui::View<GuiMessage> {
    browser_context_menu::overlay(menu, false)
}

pub(in crate::native_app) fn browser_context_menu_overlay_with_harvest_active(
    menu: &BrowserContextMenu,
) -> ui::View<GuiMessage> {
    browser_context_menu::overlay(menu, true)
}

pub(in crate::native_app) fn waveform_context_menu_overlay(
    menu: &WaveformContextMenu,
) -> ui::View<GuiMessage> {
    waveform_context_menu::overlay(menu)
}
