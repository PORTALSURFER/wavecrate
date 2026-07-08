use radiant::prelude as ui;

use super::identity::{
    RETAINED_COLLECTION_ROW_INPUT_SCOPE, retained_collection_rename_row_key,
    retained_collection_row_key,
};
use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_underlay;
use crate::native_app::app_chrome::view_models::library_sidebar::CollectionRowViewModel;
use crate::native_app::sample_library::folder_browser::commands::FolderBrowserMessage;
use crate::native_app::sample_library::folder_browser::view_contract::{
    COLLECTION_ROW_HEIGHT, SampleCollectionView,
};
use wavecrate::sample_sources::SampleCollection;

pub(super) fn collection_row(row: &CollectionRowViewModel) -> ui::View<GuiMessage> {
    let collection = &row.collection;
    let collection_id = collection.collection;
    if let Some(rename) = &row.rename {
        return ui::row([
            collection_swatch(collection.color)
                .width(34.0)
                .height(COLLECTION_ROW_HEIGHT),
            ui::text_input(rename.draft.clone())
                .selection(rename.selection_start, rename.selection_end)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
                })
                .id(rename.input_id)
                .fill_width()
                .height(COLLECTION_ROW_HEIGHT),
        ])
        .key(retained_collection_rename_row_key(collection_id))
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
        .spacing(2.0);
    }
    collection_input(collection_id, collection_visual(collection), collection)
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
}

/// Builds the transparent interaction layer for a collection row.
fn collection_input(
    collection_id: SampleCollection,
    visual: ui::View<GuiMessage>,
    collection: &SampleCollectionView,
) -> ui::View<GuiMessage> {
    sidebar_row_underlay(visual)
        .tracked_drop_target(collection.drag_active, collection.drop_target)
        .stable_row_identity(
            RETAINED_COLLECTION_ROW_INPUT_SCOPE,
            retained_collection_row_key(collection_id),
        )
        .selected(collection.selected)
        .actions(collection_row_actions(collection_id))
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
}

fn collection_row_actions(
    collection_id: SampleCollection,
) -> ui::InteractiveRowActions<GuiMessage> {
    ui::row_actions()
        .drop_target_key(
            collection_id,
            |collection_id| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnCollection(collection_id))
            },
            |collection_id, position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::HoverCollectionDropTarget(
                    collection_id,
                    position,
                ))
            },
        )
        .primary_key(collection_id, |collection_id| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection_id))
        })
        .double_key(collection_id, |collection_id| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection_id))
        })
        .secondary_key(collection_id, |collection_id, position| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::OpenCollectionContextMenu(
                collection_id,
                position,
            ))
        })
}

/// Builds the visible collection row contents above the input layer.
fn collection_visual(collection: &SampleCollectionView) -> ui::View<GuiMessage> {
    let label = format!("{}  {}", collection.hotkey, collection.name);
    ui::row([
        collection_swatch(collection.color).width(16.0),
        ui::text_line(label, COLLECTION_ROW_HEIGHT),
        collection_count(collection.assigned_count),
    ])
    .padding_x(6.0)
    .fill_width()
    .height(COLLECTION_ROW_HEIGHT)
    .spacing(0.0)
}

/// Builds the reusable collection color swatch.
fn collection_swatch(color: ui::Rgba8) -> ui::View<GuiMessage> {
    ui::color_marker(Some(color))
        .view()
        .width(16.0)
        .height(COLLECTION_ROW_HEIGHT)
}

/// Builds the fixed-width assigned-sample count cell.
pub(super) fn collection_count(count: usize) -> ui::View<GuiMessage> {
    if count == 0 {
        return ui::empty().intrinsic();
    }
    ui::text(count.to_string())
        .align_text(ui::TextAlign::Right)
        .text_color(ui::TextColorRole::Muted)
        .width(28.0)
        .height(COLLECTION_ROW_HEIGHT)
}
