use radiant::prelude as ui;

use super::SETTINGS_SIDEBAR_WIDTH;
use crate::native_app::app::{AppSettingsTab, GuiMessage, SettingsMessage};
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

pub(super) fn settings_sidebar(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    ui::column([
        ui::text_line("Settings", 24.0).key("settings-sidebar-title"),
        settings_tab_button("General", AppSettingsTab::General, snapshot.tab),
        settings_tab_button("Audio Engine", AppSettingsTab::AudioEngine, snapshot.tab),
        ui::spacer().fill_width().fill_height(),
    ])
    .key("settings-sidebar")
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
    .padding(6.0)
    .spacing(4.0)
    .width(SETTINGS_SIDEBAR_WIDTH)
    .fill_height()
}

fn settings_tab_button(
    label: &'static str,
    tab: AppSettingsTab,
    selected: AppSettingsTab,
) -> ui::View<GuiMessage> {
    let style = if tab == selected {
        ui::WidgetStyle::strong(ui::WidgetTone::Accent)
    } else {
        ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
    };
    ui::button(label)
        .style(style)
        .message(GuiMessage::Settings(SettingsMessage::SelectSettingsTab(
            tab,
        )))
        .key(format!("settings-tab-{label}"))
        .fill_width()
        .height(28.0)
}
