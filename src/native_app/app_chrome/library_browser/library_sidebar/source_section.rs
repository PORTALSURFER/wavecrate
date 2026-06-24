use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::SourceSelectorViewModel;

mod identity;
mod rows;
#[cfg(test)]
mod tests;

use rows::{source_add_button, source_row};

pub(super) fn source_selector(model: &SourceSelectorViewModel) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            ui::text("Sources").height(20.0).fill_width(),
            source_add_button(),
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
