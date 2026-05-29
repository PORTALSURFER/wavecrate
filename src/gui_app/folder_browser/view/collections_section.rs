use radiant::{
    prelude as ui,
    widgets::{WidgetStyle, WidgetTone},
};

use super::super::{
    CollectionHitMessage, CollectionHitTarget, FolderBrowserMessage, FolderBrowserState,
    GuiMessage, SampleCollectionView, collections::COLLECTION_ROW_HEIGHT,
};
use super::sidebar_panel;

pub(super) fn collections_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let rows = state
        .visible_collections()
        .into_iter()
        .map(|collection| collection_row(state, collection))
        .collect::<Vec<_>>();
    sidebar_panel(
        ui::column([
            ui::row([
                ui::text("Collections").height(20.0).fill_width(),
                ui::drag_handle_mapped(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeCollectionsPanel(message))
                })
                .key("collections-resize-handle")
                .size(26.0, 18.0),
            ])
            .spacing(4.0)
            .height(20.0)
            .fill_width(),
            ui::scroll(ui::column(rows).spacing(1.0).fill_width().height(
                COLLECTION_ROW_HEIGHT * wavecrate::sample_sources::SampleCollection::COUNT as f32,
            ))
            .style(WidgetStyle {
                tone: WidgetTone::Neutral,
                prominence: ui::WidgetProminence::Subtle,
            })
            .fill_width()
            .fill_height(),
        ])
        .spacing(4.0)
        .fill_width()
        .fill_height(),
        state.collections_panel_height(),
    )
}

fn collection_row(
    state: &FolderBrowserState,
    collection: SampleCollectionView,
) -> ui::View<GuiMessage> {
    let collection_id = collection.collection;
    if let Some(rename) = state.collection_rename_view(collection_id) {
        let caret = rename.draft.chars().count();
        return ui::row([
            ui::custom_widget(CollectionHitTarget::new(&collection), |_| None)
                .key(format!(
                    "collection-rename-swatch-{}",
                    collection.collection.index()
                ))
                .width(34.0)
                .height(COLLECTION_ROW_HEIGHT),
            ui::text_input(rename.draft)
                .selection(0, caret)
                .message_event(|message| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameInput(message))
                })
                .id(rename.input_id)
                .key(format!(
                    "collection-rename-input-{}",
                    collection.collection.index()
                ))
                .fill_width()
                .height(COLLECTION_ROW_HEIGHT),
        ])
        .key(format!(
            "collection-rename-row-{}",
            collection.collection.index()
        ))
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
        .spacing(2.0);
    }
    ui::custom_widget_mapped(
        CollectionHitTarget::new(&collection),
        move |message| match message {
            CollectionHitMessage::Activate => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection_id))
            }
            CollectionHitMessage::Rename => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection_id))
            }
            CollectionHitMessage::Drop => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnCollection(collection_id))
            }
            CollectionHitMessage::HoverDropTarget(position) => GuiMessage::FolderBrowser(
                FolderBrowserMessage::HoverCollectionDropTarget(collection_id, position),
            ),
        },
    )
    .key(format!("collection-row-{}", collection.collection.index()))
    .fill_width()
    .height(COLLECTION_ROW_HEIGHT)
}
