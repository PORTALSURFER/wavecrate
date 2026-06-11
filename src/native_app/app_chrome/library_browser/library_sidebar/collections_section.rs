use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_underlay;
use crate::native_app::sample_library::folder_browser::view_contract::{
    COLLECTION_ROW_HEIGHT, COLLECTION_ROW_SPACING, COLLECTIONS_PANEL_HEADER_CONTENT_SPACING,
    COLLECTIONS_PANEL_HEADER_HEIGHT, COLLECTIONS_PANEL_PADDING, SampleCollectionView,
};
use crate::native_app::sample_library::folder_browser::{FolderBrowserMessage, FolderBrowserState};
use crate::native_app::ui::ids as widget_ids;

const COLLECTION_ROW_INPUT_SCOPE: u64 = 0x5743_0000_0000_4c01;
/// Stable layout node id for collection-panel resize regression coverage.
const COLLECTIONS_SECTION_NODE_ID: u64 = widget_ids::COLLECTIONS_SECTION_NODE_ID;
/// Stable layout node id for the collection rows scroll viewport.
const COLLECTIONS_LIST_SCROLL_NODE_ID: u64 = widget_ids::COLLECTIONS_LIST_SCROLL_NODE_ID;

pub(super) fn collections_section(state: &FolderBrowserState) -> ui::View<GuiMessage> {
    let rows = state
        .visible_collections()
        .into_iter()
        .map(|collection| collection_row(state, collection))
        .collect::<Vec<_>>();
    ui::panel_section_from_parts(
        ui::PanelSectionParts::new(
            "Collections",
            ui::scroll(
                ui::column(rows)
                    .spacing(COLLECTION_ROW_SPACING)
                    .fill_width()
                    .height(state.collections_list_height()),
            )
            .style(ui::WidgetStyle::subtle(ui::WidgetTone::Neutral))
            .id(COLLECTIONS_LIST_SCROLL_NODE_ID)
            .fill_width()
            .fill_height(),
        )
        .trailing_resize_handle("collections-resize-handle", |message| {
            GuiMessage::FolderBrowser(FolderBrowserMessage::ResizeCollectionsPanel(message))
        })
        .padding(COLLECTIONS_PANEL_PADDING)
        .spacing(COLLECTIONS_PANEL_HEADER_CONTENT_SPACING)
        .title_height(COLLECTIONS_PANEL_HEADER_HEIGHT)
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
        return ui::row([
            collection_swatch(collection.color)
                .key(format!(
                    "collection-rename-swatch-{}",
                    collection.collection.index()
                ))
                .width(34.0)
                .height(COLLECTION_ROW_HEIGHT),
            ui::text_input(rename.draft)
                .selection(rename.selection_start, rename.selection_end)
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
    sidebar_row_underlay(visual)
        .tracked_drop_target(collection.drag_active, collection.drop_target)
        .stable_u64_input_id(COLLECTION_ROW_INPUT_SCOPE, collection_id.index() as u64)
        .selected(collection.selected)
        .actions(
            ui::InteractiveRowActions::new()
                .drop_target_key(
                    collection_id,
                    |collection_id| {
                        GuiMessage::FolderBrowser(FolderBrowserMessage::DropOnCollection(
                            collection_id,
                        ))
                    },
                    |collection_id, position| {
                        GuiMessage::FolderBrowser(FolderBrowserMessage::HoverCollectionDropTarget(
                            collection_id,
                            position,
                        ))
                    },
                )
                .activate_key(collection_id, |collection_id| {
                    GuiMessage::FolderBrowser(FolderBrowserMessage::ActivateCollection(
                        collection_id,
                    ))
                })
                .double_activate_key(collection_id, |collection_id| {
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
        return ui::empty().intrinsic();
    }
    ui::text(count.to_string())
        .align_text(ui::TextAlign::Right)
        .text_color(ui::TextColorRole::Muted)
        .width(28.0)
        .height(COLLECTION_ROW_HEIGHT)
}

/// Collection-section view tests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::{
        SIDEBAR_ROW_DROP_TARGET_FILL, SIDEBAR_ROW_HOVER_FILL, sidebar_row_palette,
    };
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
                ui::stable_widget_id_u64(COLLECTION_ROW_INPUT_SCOPE, collection_id.index() as u64),
                ui::WidgetOutput::typed(GuiMessage::FolderBrowser(
                    FolderBrowserMessage::RenameCollection(collection_id)
                )),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::RenameCollection(collection)
            )) if collection == collection_id
        ));
    }

    #[test]
    /// Verifies active drop targets keep a distinct visual state.
    fn collection_input_paints_current_drop_target_fill() {
        let collection = collection_view(true, true);
        let collection_id = collection.collection;
        let frame = collection_input(collection_id, collection_visual(&collection), &collection)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, COLLECTION_ROW_HEIGHT));

        assert!(frame.paint_plan.fill_rects().any(|fill| {
            fill.color == SIDEBAR_ROW_DROP_TARGET_FILL
                && fill.rect.width() == 180.0
                && fill.rect.height() == COLLECTION_ROW_HEIGHT
        }));
    }

    #[test]
    fn collection_rows_use_shared_grey_sidebar_hover_fill() {
        assert_eq!(
            sidebar_row_palette(true).hovered,
            Some(SIDEBAR_ROW_HOVER_FILL)
        );
    }

    #[test]
    fn collection_drop_target_keeps_distinct_target_fill() {
        assert_eq!(
            sidebar_row_palette(true).active_target,
            Some(SIDEBAR_ROW_DROP_TARGET_FILL)
        );
    }

    #[test]
    /// Empty collections should not paint a count placeholder rectangle.
    fn collection_count_hides_empty_placeholder() {
        let frame = collection_count(0)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(28.0, COLLECTION_ROW_HEIGHT));

        assert!(frame.paint_plan.fill_rects().next().is_none());
        assert!(frame.paint_plan.stroke_rects().next().is_none());
        assert!(frame.paint_plan.text_runs().next().is_none());
    }

    #[test]
    /// Empty collections should not reserve trailing count-cell layout space.
    fn collection_count_collapses_empty_placeholder_layout() {
        const EMPTY_COUNT_NODE_ID: u64 = widget_ids::EMPTY_COLLECTION_COUNT_NODE_ID;
        let layout = ui::row([
            ui::text_line("Collection 1", COLLECTION_ROW_HEIGHT),
            collection_count(0).id(EMPTY_COUNT_NODE_ID),
        ])
        .view_layout_at_size(ui::Vector2::new(240.0, COLLECTION_ROW_HEIGHT));
        let rect = layout
            .rects
            .get(&EMPTY_COUNT_NODE_ID)
            .expect("empty collection count layout rect");

        assert_eq!(rect.width(), 0.0);
    }

    #[test]
    /// Non-empty collections still paint the numeric count.
    fn collection_count_shows_assigned_count() {
        let frame = collection_count(3)
            .view_frame_at_size_with_default_theme(ui::Vector2::new(28.0, COLLECTION_ROW_HEIGHT));

        assert!(frame.paint_plan.contains_text("3"));
    }

    #[test]
    /// Verifies the section keeps its resized height in the parent layout slot.
    fn collections_section_layout_uses_configured_height() {
        let mut state = FolderBrowserState::load_default();
        state.resize_collections_panel(ui::DragHandleMessage::started(ui::Point::new(0.0, 200.0)));
        state.resize_collections_panel(ui::DragHandleMessage::moved(ui::Point::new(0.0, 120.0)));

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

    #[test]
    /// Verifies the collection list reserves exactly the useful row stack height.
    fn collections_section_list_height_matches_visible_rows() {
        let state = FolderBrowserState::load_default();
        let expected = COLLECTION_ROW_HEIGHT * state.visible_collections().len() as f32
            + COLLECTION_ROW_SPACING * (state.visible_collections().len() - 1) as f32;

        assert_eq!(state.collections_list_height(), expected);
    }

    #[test]
    /// The useful maximum height should fit all collection rows without residual scroll.
    fn collections_section_max_height_does_not_overflow_rows() {
        let state = FolderBrowserState::load_default();
        let layout = collections_section(&state).view_layout_at_size(ui::Vector2::new(
            240.0,
            state.max_collections_panel_height(),
        ));

        assert!(
            !layout
                .overflow_flags
                .get(&COLLECTIONS_LIST_SCROLL_NODE_ID)
                .is_some_and(|overflow| overflow.y),
            "collections list should not have vertical scroll overflow at max height"
        );
    }
}
