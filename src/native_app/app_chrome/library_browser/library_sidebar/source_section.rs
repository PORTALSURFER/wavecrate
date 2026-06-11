use radiant::prelude as ui;

use crate::native_app::app::GuiMessage;
use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::sidebar_row_underlay;
use crate::native_app::app_chrome::view_models::library_sidebar::{
    SourceRowViewModel, SourceSelectorViewModel,
};
use crate::native_app::sample_library::folder_browser::FolderBrowserMessage;

const SOURCE_ROW_INPUT_SCOPE: u64 = 0x5743_0000_0000_5301;
const ACTIVE_SOURCE_MARKER_COLOR: ui::Rgba8 = ui::Rgba8::new(255, 82, 62, 245);
const ACTIVE_SOURCE_MARKER_WIDTH: f32 = 3.0;

pub(super) fn source_selector(model: &SourceSelectorViewModel) -> ui::View<GuiMessage> {
    ui::column([
        ui::row([
            ui::text("Sources").height(20.0).fill_width(),
            ui::button("+")
                .primary()
                .message(GuiMessage::FolderBrowser(FolderBrowserMessage::AddSource))
                .key("source-add-button")
                .size(28.0, 22.0),
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

fn source_row(source: &SourceRowViewModel) -> ui::View<GuiMessage> {
    let row_key = source.id.clone();
    let label = if source.scanning {
        format!("{} (scanning)", source.label)
    } else {
        source.label.clone()
    };
    let visual = source_row_content(label);
    sidebar_row_underlay(visual)
        .stable_input_id(SOURCE_ROW_INPUT_SCOPE, source.id.as_str())
        .selected(source.selected)
        .leading_marker_if(source.selected, active_source_marker())
        .actions(ui::InteractiveRowActions::new().activate_secondary_key(
            source.id.clone(),
            |source_id| GuiMessage::FolderBrowser(FolderBrowserMessage::SelectSource(source_id)),
            |source_id, position| {
                GuiMessage::FolderBrowser(FolderBrowserMessage::OpenSourceContextMenu(
                    source_id, position,
                ))
            },
        ))
        .key(format!("source-row-{row_key}"))
        .fill_width()
        .height(24.0)
}

fn source_row_content(label: String) -> ui::View<GuiMessage> {
    ui::text_line(label, 24.0).padding_x(8.0)
}

fn active_source_marker() -> ui::DenseRowMarkerStyle {
    ui::DenseRowMarkerStyle::new(
        ui::DenseRowMarkerParts::leading(ACTIVE_SOURCE_MARKER_WIDTH)
            .edge_inset(4.0)
            .vertical_inset(4.0),
        ACTIVE_SOURCE_MARKER_COLOR,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::app_chrome::library_browser::library_sidebar::sidebar_row::{
        SIDEBAR_ROW_HOVER_FILL, sidebar_row_palette,
    };
    use crate::native_app::sample_library::folder_browser::{
        FolderBrowserState, model::SourceEntry,
    };
    use radiant::prelude::IntoView;

    fn test_source(id: &str) -> SourceEntry {
        SourceEntry::new(id, "Source", std::path::PathBuf::from("C:/samples"))
    }

    #[test]
    fn source_row_routes_primary_activation_through_interactive_row() {
        let source = test_source("source-a");
        let state =
            FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
        let model = SourceSelectorViewModel::from_folder_browser(&state);
        let row = model.rows.first().expect("source row");

        assert_eq!(
            source_row(row).view_dispatch_widget_output(
                ui::stable_widget_id(SOURCE_ROW_INPUT_SCOPE, source.id.as_str()),
                ui::WidgetOutput::typed(GuiMessage::FolderBrowser(
                    FolderBrowserMessage::SelectSource(source.id.clone())
                )),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::SelectSource(source.id.clone())
            ))
        );
    }

    #[test]
    fn source_row_routes_secondary_activation_to_context_menu() {
        let source = test_source("source-b");
        let state =
            FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
        let position = ui::Point::new(12.0, 20.0);
        let model = SourceSelectorViewModel::from_folder_browser(&state);
        let row = model.rows.first().expect("source row");

        assert_eq!(
            source_row(row).view_dispatch_widget_output(
                ui::stable_widget_id(SOURCE_ROW_INPUT_SCOPE, source.id.as_str()),
                ui::WidgetOutput::typed(GuiMessage::FolderBrowser(
                    FolderBrowserMessage::OpenSourceContextMenu(source.id.clone(), position)
                )),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::OpenSourceContextMenu(source.id.clone(), position)
            ))
        );
    }

    #[test]
    fn selected_source_row_paints_left_active_marker() {
        let source = test_source("source-active");
        let state =
            FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());
        let model = SourceSelectorViewModel::from_folder_browser(&state);
        let row = model.rows.first().expect("source row");
        let frame =
            source_row(row).view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, 24.0));

        assert!(
            frame.paint_plan.fill_rects().any(|fill| {
                fill.color == ACTIVE_SOURCE_MARKER_COLOR
                    && fill.rect.width() == ACTIVE_SOURCE_MARKER_WIDTH
                    && fill.rect.height() == 16.0
                    && fill.rect.min.x == 4.0
                    && fill.rect.min.y == 4.0
            }),
            "selected source should paint a clear left active marker"
        );
    }

    #[test]
    fn inactive_source_row_does_not_paint_active_marker() {
        let source = test_source("source-inactive");
        let selected = test_source("source-active");
        let state = FolderBrowserState::from_sources_deferred(
            vec![source.clone(), selected.clone()],
            selected.id.clone(),
        );
        let model = SourceSelectorViewModel::from_folder_browser(&state);
        let row = model.rows.first().expect("source row");
        let frame =
            source_row(row).view_frame_at_size_with_default_theme(ui::Vector2::new(180.0, 24.0));

        assert!(
            !frame
                .paint_plan
                .fill_rects()
                .any(|fill| fill.color == ACTIVE_SOURCE_MARKER_COLOR),
            "inactive sources should stay visually quiet"
        );
    }

    #[test]
    fn source_rows_use_shared_grey_sidebar_hover_fill() {
        assert_eq!(
            sidebar_row_palette(true).hovered,
            Some(SIDEBAR_ROW_HOVER_FILL)
        );
    }
}
