use radiant::prelude as ui;

use super::super::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, SampleCollectionView,
    collections::COLLECTION_ROW_HEIGHT,
};

const COLLECTION_ROW_INPUT_SCOPE: u64 = 0x5743_0000_0000_4c01;
/// Stable layout node id for collection-panel resize regression coverage.
const COLLECTIONS_SECTION_NODE_ID: u64 = 0x5743_0000_0000_4c02;

pub(super) fn collections_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let rows = state
        .visible_collections()
        .into_iter()
        .map(|collection| collection_row(state, collection))
        .collect::<Vec<_>>();
    ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Collections",
            ui::scroll(ui::column(rows).spacing(1.0).fill_width().height(
                COLLECTION_ROW_HEIGHT * wavecrate::sample_sources::SampleCollection::COUNT as f32,
            ))
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
            .fill_width()
            .fill_height(),
        )
        .trailing(
            ui::drag_handle_mapped(|message| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeCollectionsPanel(message))
            })
            .key("collections-resize-handle")
            .size(26.0, 18.0),
        )
        .height(state.collections_panel_height()),
    )
    .id(COLLECTIONS_SECTION_NODE_ID)
    .fill_width()
}

fn collection_row(
    state: &FolderBrowserState,
    collection: SampleCollectionView,
) -> ui::View<GuiMessage> {
    let collection_id = collection.collection;
    if let Some(rename) = state.collection_rename_view(collection_id) {
        let caret = rename.draft.chars().count();
        return ui::row([
            collection_swatch(collection.color)
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
    collection_input(collection_id, collection_visual(&collection), &collection)
        .key(format!("collection-row-{}", collection.collection.index()))
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
}

/// Builds the transparent interaction layer for a collection row.
fn collection_input(
    collection_id: wavecrate::sample_sources::SampleCollection,
    visual: ui::View<GuiMessage>,
    collection: &SampleCollectionView,
) -> ui::View<GuiMessage> {
    ui::interactive_row_underlay(visual)
        .row(|row| row.tracked_drop_target(collection.drag_active, collection.drop_target))
        .input_id(collection_row_input_id(collection_id))
        .style(collection_input_style(collection))
        .actions(
            ui::InteractiveRowActions::new()
                .drop(move || {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnCollection(collection_id))
                })
                .hover_drop(move |position| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::HoverCollectionDropTarget(
                        collection_id,
                        position,
                    ))
                })
                .activate(move || {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(
                        collection_id,
                    ))
                })
                .double_activate(move || {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection_id))
                }),
        )
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
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
fn collection_count(count: usize) -> ui::View<GuiMessage> {
    if count == 0 {
        return ui::spacer().width(28.0).height(COLLECTION_ROW_HEIGHT);
    }
    ui::text(count.to_string())
        .align_text(ui::TextAlign::Right)
        .text_color(ui::TextColorRole::Muted)
        .width(28.0)
        .height(COLLECTION_ROW_HEIGHT)
}

/// Resolves selected and drop-target visual state for the input layer.
fn collection_input_style(collection: &SampleCollectionView) -> ui::WidgetStyle {
    let tone = if collection.drop_target {
        ui::WidgetTone::Warning
    } else {
        ui::WidgetTone::Accent
    };
    if collection.selected || collection.drop_target {
        ui::WidgetStyle::strong(tone)
    } else {
        ui::WidgetStyle::subtle(tone)
    }
}

/// Returns the stable input widget id for a collection row.
fn collection_row_input_id(collection: wavecrate::sample_sources::SampleCollection) -> u64 {
    ui::stable_widget_id_u64(COLLECTION_ROW_INPUT_SCOPE, collection.index() as u64)
}

/// Collection-section view tests.
#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;
    use wavecrate::sample_sources::SampleCollection;

    /// Builds a minimal collection view for interaction tests.
    fn collection_view(drag_active: bool, drop_target: bool) -> SampleCollectionView {
        let collection = SampleCollection::new(0).expect("valid collection");
        SampleCollectionView {
            collection,
            hotkey: '1',
            name: String::from("Collection 1"),
            color: ui::Rgba8 {
                r: 255,
                g: 86,
                b: 98,
                a: 240,
            },
            selected: false,
            drop_target,
            drag_active,
            assigned_count: 0,
        }
    }

    #[test]
    /// Verifies double-click routing through Radiant's generic row primitive.
    fn collection_input_routes_double_click_to_rename() {
        let collection = collection_view(false, false);
        let collection_id = collection.collection;

        assert!(matches!(
            collection_input(collection_id, collection_visual(&collection), &collection)
                .view_dispatch_widget_output(
                collection_row_input_id(collection_id),
                ui::WidgetOutput::typed(ui::InteractiveRowMessage::DoubleActivate),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::RenameCollection(collection)
            )) if collection == collection_id
        ));
    }

    #[test]
    /// Verifies active drop targets suppress redundant hover-drop messages.
    fn collection_input_uses_drop_only_for_current_drop_target() {
        let collection = collection_view(true, true);
        let collection_id = collection.collection;
        let surface = collection_input(collection_id, collection_visual(&collection), &collection)
            .into_surface();
        let widget = surface
            .find_widget(collection_row_input_id(collection_id))
            .and_then(|widget| {
                widget
                    .widget()
                    .as_any()
                    .downcast_ref::<ui::InteractiveRowWidget>()
            })
            .expect("collection row input widget");

        assert!(widget.props.droppable);
        assert!(!widget.props.drop_hover);
    }

    #[test]
    /// Verifies the section keeps its resized height in the parent layout slot.
    fn collections_section_layout_uses_configured_height() {
        let mut state = FolderBrowserState::load_default();
        state.resize_collections_panel(ui::DragHandleMessage::Started {
            position: ui::Point::new(0.0, 200.0),
        });
        state.resize_collections_panel(ui::DragHandleMessage::Moved {
            position: ui::Point::new(0.0, 120.0),
        });

        let layout = ui::column([
            collections_section(&state),
            ui::spacer().fill_width().fill_height(),
        ])
        .view_layout_at_size(ui::Vector2::new(240.0, 600.0));
        let section = layout
            .rects
            .get(&COLLECTIONS_SECTION_NODE_ID)
            .expect("collections section layout rect");

        assert_eq!(section.height(), state.collections_panel_height());
    }
}
