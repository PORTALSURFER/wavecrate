use radiant::prelude as ui;

pub(super) fn sidebar_row_style() -> ui::WidgetStyle {
    ui::WidgetStyle::subtle(ui::WidgetTone::Accent)
}

pub(super) fn active_sidebar_row_style() -> ui::WidgetStyle {
    ui::WidgetStyle::strong(ui::WidgetTone::Accent)
}
