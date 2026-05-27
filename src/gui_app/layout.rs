use super::{GuiAppState, GuiMessage};
use crate::gui_app::{
    audio_settings::top_status_bar, context_menu, folder_browser,
    metadata_tags::MetadataTagCategoryGroup, sample_browser_view::sample_browser, status_bar,
    toolbar::main_toolbar, waveform_panel::waveform_panel,
};
use radiant::prelude as ui;

pub(super) fn view(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let content = ui::column([
        top_status_bar(state),
        center_panel(state),
        status_bar::bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
    if state.job_details_open {
        if let Some(progress) = state.folder_progress.as_ref() {
            layers.push(status_bar::job_details_popover(progress));
        }
    }
    if let Some(menu) = state.context_menu.as_ref() {
        layers.push(context_menu::overlay(menu));
    }
    if layers.len() > 1 {
        ui::stack(layers).fill()
    } else {
        layers.remove(0)
    }
}

fn center_panel(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let mut children = vec![folder_sidebar(state)];
    if state.metadata_tag_library_open && state.folder_browser.selected_file_id().is_some() {
        children.push(metadata_tag_library_panel(state));
    }
    children.push(folder_splitter());
    children.push(main_area(state));
    ui::row(children).padding(6.0).fill()
}

fn folder_sidebar(state: &GuiAppState) -> ui::View<GuiMessage> {
    folder_browser::folder_browser_view(
        &state.folder_browser,
        state.folder_width,
        state.folder_browser.selected_file_id().is_some(),
        state.metadata_tag_draft.as_str(),
        state.metadata_tag_tokens.as_slice(),
        state.metadata_tag_suggestion().as_deref(),
        state.selected_metadata_tags(),
    )
    .width(state.folder_width)
    .fill_height()
}

fn metadata_tag_library_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
    let selected_tags = state.selected_metadata_tags();
    let groups = state
        .categorized_metadata_tags()
        .into_iter()
        .map(|group| metadata_tag_category_group(group, selected_tags))
        .collect::<Vec<_>>();
    ui::column([
        ui::row([
            ui::text("Tag Editor").height(22.0).fill_width(),
            ui::button("x")
                .message(GuiMessage::ToggleMetadataTagLibrary)
                .key("metadata-tag-library-close")
                .size(22.0, 20.0),
        ])
        .spacing(4.0)
        .fill_width()
        .height(24.0),
        ui::scroll(ui::column(groups).spacing(3.0).fill_width())
            .fill_width()
            .fill_height(),
    ])
    .key("metadata-tag-library-panel")
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Neutral,
        prominence: ui::WidgetProminence::Subtle,
    })
    .padding(6.0)
    .spacing(4.0)
    .width(220.0)
    .fill_height()
}

fn metadata_tag_category_group(
    group: MetadataTagCategoryGroup,
    selected_tags: &[String],
) -> ui::View<GuiMessage> {
    let disclosure = if group.collapsed { ">" } else { "v" };
    let count_label = if group.tags.is_empty() {
        String::new()
    } else {
        format!(" ({})", group.tags.len())
    };
    let mut children = vec![
        ui::button(format!("{disclosure} {}{count_label}", group.label))
            .message(GuiMessage::ToggleMetadataTagCategory(group.id.to_string()))
            .key(format!("metadata-tag-category-{}", group.id))
            .subtle()
            .fill_width()
            .height(22.0),
    ];
    if !group.collapsed {
        if group.tags.is_empty() {
            children.push(
                ui::text("No tags yet")
                    .height(20.0)
                    .fill_width()
                    .truncate()
                    .padding(4.0),
            );
        } else {
            children.extend(
                group
                    .tags
                    .into_iter()
                    .map(|tag| metadata_tag_library_row(tag, selected_tags)),
            );
        }
    }
    ui::column(children)
        .key(format!("metadata-tag-category-group-{}", group.id))
        .spacing(2.0)
        .fill_width()
}

fn metadata_tag_library_row(tag: String, selected_tags: &[String]) -> ui::View<GuiMessage> {
    let selected = selected_tags.iter().any(|selected| selected == &tag);
    let label = if selected {
        format!("[x] {tag}")
    } else {
        format!("[ ] {tag}")
    };
    let mut button = ui::button(label)
        .message(GuiMessage::ToggleMetadataTag(tag.clone()))
        .key(format!("metadata-tag-library-row-{tag}"))
        .fill_width()
        .height(22.0);
    if selected {
        button = button.primary();
    } else {
        button = button.subtle();
    }
    button
}

fn folder_splitter() -> ui::View<GuiMessage> {
    ui::drag_handle()
        .mapped(GuiMessage::ResizeFolder)
        .key("folder-browser-splitter-handle")
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .width(11.0)
        .fill_height()
        .padding(2.0)
}

fn main_area(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    ui::column([
        main_toolbar(state),
        waveform_panel(state),
        sample_browser(state, state.folder_resize.is_some()),
    ])
    .padding(4.0)
    .fill()
}
