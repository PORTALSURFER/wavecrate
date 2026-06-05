use radiant::prelude as ui;

use super::super::{FolderBrowserMessage, FolderBrowserState, GuiMessage, SourceEntry};

const SOURCE_ROW_INPUT_SCOPE: u64 = 0x5743_0000_0000_5301;

pub(super) fn source_selector(state: &FolderBrowserState) -> ui::View<GuiMessage> {
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
        ui::column(
            state
                .sources
                .iter()
                .map(|source| source_row(state, source))
                .collect::<Vec<_>>(),
        )
        .spacing(2.0)
        .fill_width(),
    ])
    .spacing(3.0)
    .fill_width()
}

fn source_row(state: &FolderBrowserState, source: &SourceEntry) -> ui::View<GuiMessage> {
    let row_key = source.id.clone();
    let selected = state.selected_source == source.id;
    let label = if source.loading_task.is_some() {
        format!("{} (scanning)", source.label)
    } else {
        source.label.clone()
    };
    let visual = ui::text_line(label, 24.0).padding_x(8.0);
    ui::interactive_row_underlay(visual)
        .input_id(source_row_input_id(source.id.as_str()))
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
        .style(source_row_style(selected))
        .fill_width()
        .height(24.0)
}

fn source_row_style(selected: bool) -> ui::WidgetStyle {
    if selected {
        ui::WidgetStyle::subtle(ui::WidgetTone::Accent)
    } else {
        ui::WidgetStyle::default()
    }
}

fn source_row_input_id(source_id: &str) -> u64 {
    ui::stable_widget_id(SOURCE_ROW_INPUT_SCOPE, source_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::prelude::IntoView;

    fn test_source(id: &str) -> SourceEntry {
        SourceEntry::new(id, "Source", std::path::PathBuf::from("C:/samples"))
    }

    #[test]
    fn source_row_routes_primary_activation_through_interactive_row() {
        let source = test_source("source-a");
        let state =
            FolderBrowserState::from_sources_deferred(vec![source.clone()], source.id.clone());

        assert_eq!(
            source_row(&state, &source).view_dispatch_widget_output(
                source_row_input_id(source.id.as_str()),
                ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
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

        assert_eq!(
            source_row(&state, &source).view_dispatch_widget_output(
                source_row_input_id(source.id.as_str()),
                ui::WidgetOutput::typed(ui::InteractiveRowMessage::SecondaryActivate { position }),
            ),
            Some(GuiMessage::FolderBrowser(
                FolderBrowserMessage::OpenSourceContextMenu(source.id.clone(), position)
            ))
        );
    }
}
