use radiant::prelude as ui;

mod projection;

use self::projection::{SettingsTabProjection, settings_sidebar_projection};
use super::SETTINGS_SIDEBAR_WIDTH;
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::settings::AudioSettingsSnapshot;

pub(super) fn settings_sidebar(snapshot: &AudioSettingsSnapshot) -> ui::View<GuiMessage> {
    let projection = settings_sidebar_projection(snapshot);
    let mut rows = Vec::with_capacity(projection.tabs.len() + 2);
    rows.push(ui::text_line(projection.title, 24.0));
    rows.extend(projection.tabs.into_iter().map(settings_tab_button));
    rows.push(ui::spacer().fill_width().fill_height());

    ui::column(rows)
        .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
        .padding(6.0)
        .spacing(4.0)
        .width(SETTINGS_SIDEBAR_WIDTH)
        .fill_height()
}

fn settings_tab_button(projection: SettingsTabProjection) -> ui::View<GuiMessage> {
    let style = if projection.selected {
        ui::WidgetStyle::strong(ui::WidgetTone::Accent)
    } else {
        ui::WidgetStyle::subtle(ui::WidgetTone::Neutral)
    };
    ui::button(projection.label)
        .style(style)
        .message(GuiMessage::Settings(projection.message))
        .fill_width()
        .height(28.0)
}
