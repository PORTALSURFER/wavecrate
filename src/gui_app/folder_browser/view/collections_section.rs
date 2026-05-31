use radiant::{
    gui::types::Rgba8,
    prelude as ui,
    widgets::{TextColorRole, WidgetStyle},
};

use super::super::{
    FolderBrowserMessage, FolderBrowserState, GuiMessage, SampleCollectionView,
    collections::COLLECTION_ROW_HEIGHT,
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
                tone: ui::WidgetTone::Neutral,
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
    ui::stack([
        collection_input(collection_id, &collection),
        collection_visual(&collection),
    ])
    .key(format!("collection-row-{}", collection.collection.index()))
    .fill_width()
    .height(COLLECTION_ROW_HEIGHT)
}

/// Builds the transparent interaction layer for a collection row.
fn collection_input(
    collection_id: wavecrate::sample_sources::SampleCollection,
    collection: &SampleCollectionView,
) -> ui::View<GuiMessage> {
    let mut input = ui::interactive_row()
        .pointer_motion_during_interaction()
        .pointer_motion_active(collection.drop_target);
    if collection.drag_active && !collection.drop_target {
        input = input.droppable(true);
    } else if collection.drag_active {
        input = input.drop_only(true);
    }
    input
        .mapped(move |message| match message {
            ui::InteractiveRowMessage::Activate
            | ui::InteractiveRowMessage::ActivateWithModifiers { .. } => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(collection_id))
            }
            ui::InteractiveRowMessage::DoubleActivate => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::RenameCollection(collection_id))
            }
            ui::InteractiveRowMessage::Drop => {
                GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnCollection(collection_id))
            }
            ui::InteractiveRowMessage::HoverDropTarget { position } => GuiMessage::FolderBrowser(
                FolderBrowserMessage::HoverCollectionDropTarget(collection_id, position),
            ),
            ui::InteractiveRowMessage::SecondaryActivate { .. }
            | ui::InteractiveRowMessage::Drag(_) => GuiMessage::Noop,
        })
        .id(collection_row_input_id(collection_id))
        .style(collection_input_style(collection))
        .fill_width()
        .height(COLLECTION_ROW_HEIGHT)
}

/// Builds the visible collection row contents above the input layer.
fn collection_visual(collection: &SampleCollectionView) -> ui::View<GuiMessage> {
    let label = format!("{}  {}", collection.hotkey, collection.name);
    ui::row([
        collection_swatch(collection.color).width(16.0),
        ui::text(label)
            .truncate()
            .fill_width()
            .height(COLLECTION_ROW_HEIGHT),
        collection_count(collection.assigned_count),
    ])
    .padding_x(6.0)
    .fill_width()
    .height(COLLECTION_ROW_HEIGHT)
    .spacing(0.0)
}

/// Builds the reusable collection color swatch.
fn collection_swatch(color: Rgba8) -> ui::View<GuiMessage> {
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
        .text_color(TextColorRole::Muted)
        .width(28.0)
        .height(COLLECTION_ROW_HEIGHT)
}

/// Resolves selected and drop-target visual state for the input layer.
fn collection_input_style(collection: &SampleCollectionView) -> WidgetStyle {
    WidgetStyle {
        tone: if collection.drop_target {
            ui::WidgetTone::Warning
        } else {
            ui::WidgetTone::Accent
        },
        prominence: if collection.selected || collection.drop_target {
            ui::WidgetProminence::Strong
        } else {
            ui::WidgetProminence::Subtle
        },
    }
}

/// Returns the stable input widget id for a collection row.
fn collection_row_input_id(collection: wavecrate::sample_sources::SampleCollection) -> u64 {
    0x5743_0000_0000_4c00 + collection.index() as u64
}

/// Collection-section view tests.
#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{prelude::IntoView, runtime::UiSurface};
    use wavecrate::sample_sources::SampleCollection;

    /// Builds a minimal collection view for interaction tests.
    fn collection_view(drag_active: bool, drop_target: bool) -> SampleCollectionView {
        let collection = SampleCollection::new(0).expect("valid collection");
        SampleCollectionView {
            collection,
            hotkey: '1',
            name: String::from("Collection 1"),
            color: Rgba8 {
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
        let surface: UiSurface<GuiMessage> =
            UiSurface::new(collection_input(collection_id, &collection).into_node());

        assert!(matches!(
            surface.dispatch_widget_output(
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
        let surface: UiSurface<GuiMessage> =
            UiSurface::new(collection_input(collection_id, &collection).into_node());
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
}
