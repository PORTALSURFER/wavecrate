use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::SourceSelectorViewModel;

mod identity;
mod rows;
#[cfg(test)]
mod tests;

use rows::{source_add_button, source_missing_color, source_row};

pub(super) fn source_selector(model: &SourceSelectorViewModel) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            source_section_title(model),
            source_add_button(model.help_tooltips_enabled),
        ])
        .spacing(3.0)
        .fill_width()
        .height(24.0),
        ui::column(model.rows.iter().map(source_row).collect::<Vec<_>>())
            .spacing(2.0)
            .fill_width(),
    ])
    .spacing(3.0)
    .fill_width()
}

fn source_section_title(model: &SourceSelectorViewModel) -> ui::View<GuiMessage> {
    let label = match model.missing_count {
        0 => String::from("Sources"),
        1 => String::from("Sources (1 missing)"),
        count => format!("Sources ({count} missing)"),
    };
    let title = ui::text(label).height(20.0).fill_width();
    if model.missing_count > 0 {
        title.text_color(ui::TextColorRole::Custom(source_missing_color()))
    } else {
        title
    }
}
