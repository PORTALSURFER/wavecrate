use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::view_models::library_sidebar::LibrarySidebarViewModel;

mod collections_section;
mod filter_section;
mod folder_tree;
mod sidebar_row;
mod source_section;
mod tag_completion;
mod tag_editor;
mod tag_entry_layout;
#[cfg(test)]
mod test_support;

use collections_section::collections_section;
use filter_section::filter_section;
use folder_tree::folder_tree_section;
use source_section::source_selector;
use tag_editor::tag_editor_section;

pub(in crate::native_app) use tag_completion::{TAG_COMPLETION_POPUP_GAP, tag_completion_overlay};
pub(in crate::native_app) use tag_entry_layout::tag_field_content_width;
#[cfg(test)]
pub(in crate::native_app) use test_support::library_sidebar_view;

pub(in crate::native_app) fn library_sidebar(
    model: LibrarySidebarViewModel,
) -> ui::View<GuiMessage> {
    let sidebar_width = model.sidebar_width;
    library_sidebar_content(model)
        .width(sidebar_width)
        .fill_height()
}

fn library_sidebar_content(model: LibrarySidebarViewModel) -> ui::View<GuiMessage> {
    ui::column([
        source_selector(&model.source_selector),
        folder_tree_section(model.folder_tree),
        collections_section(&model.collections),
        filter_section(&model.filter),
        tag_editor_section(
            &model.tag_editor,
            model.sidebar_width,
            model.metadata_panel_height,
        ),
    ])
    .spacing(3.0)
    .fill_width()
    .padding_x(4.0)
    .style(ui::WidgetStyle::default())
    .fill_height()
}
