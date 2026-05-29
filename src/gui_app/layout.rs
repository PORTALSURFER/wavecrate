use super::{GuiAppState, GuiMessage};
use crate::gui_app::{
    audio_settings::top_status_bar, context_menu, folder_browser,
    metadata_tags::MetadataTagCategoryGroup, sample_browser_view::sample_browser, status_bar,
    toolbar::main_toolbar, waveform_panel::waveform_panel,
};
use radiant::prelude as ui;

const TAG_LIBRARY_PILL_HEIGHT: f32 = 18.0;
const TAG_LIBRARY_PILL_GAP: f32 = 3.0;

pub(super) fn view(state: &mut GuiAppState) -> ui::View<GuiMessage> {
    let content = ui::column([
        top_status_bar(state),
        center_panel(state),
        status_bar::bottom_status_bar(state),
    ])
    .spacing(0.0)
    .fill();
    let mut layers = vec![content];
    if state.job_details_open
        && let Some(progress) = state.folder_progress.as_ref()
    {
        layers.push(status_bar::job_details_popover(progress));
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
        state.pending_metadata_tag_category_tag(),
        state.metadata_tag_input_placeholder(),
        state.metadata_tag_completion_suffix().as_deref(),
        state.metadata_tag_completion_options().as_slice(),
        state.selected_metadata_tags(),
        state.selected_metadata_tag_display_categories().as_slice(),
        state.selected_metadata_tag.as_deref(),
    )
    .width(state.folder_width)
    .fill_height()
}

fn metadata_tag_library_panel(state: &GuiAppState) -> ui::View<GuiMessage> {
    let selected_tags = state.selected_metadata_tags();
    let drag_active = state.metadata_tag_drag_active();
    let drop_hover = state.metadata_tag_drop_hover();
    let groups = state
        .categorized_metadata_tags()
        .into_iter()
        .map(|group| metadata_tag_category_group(group, selected_tags, drag_active, drop_hover))
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
    drag_active: bool,
    drop_hover: Option<&str>,
) -> ui::View<GuiMessage> {
    let disclosure = if group.collapsed { ">" } else { "v" };
    let count_label = if group.tags.is_empty() {
        String::new()
    } else {
        format!(" ({})", group.tags.len())
    };
    let locked = group.locked;
    let category_id = group.id.to_string();
    let category_hovered = drop_hover == Some(group.id);
    let mut children = vec![metadata_tag_category_header(
        category_id.clone(),
        format!(
            "{disclosure} {}{count_label}{}",
            group.label,
            if locked { " [locked]" } else { "" }
        ),
        locked,
        drag_active,
        category_hovered,
    )];
    if category_hovered {
        children.push(
            ui::row(Vec::<ui::View<GuiMessage>>::new())
                .key(format!("metadata-tag-category-drop-indicator-{}", group.id))
                .style(ui::WidgetStyle {
                    tone: ui::WidgetTone::Warning,
                    prominence: ui::WidgetProminence::Strong,
                })
                .fill_width()
                .height(4.0),
        );
    }
    if !group.collapsed {
        if group.tags.is_empty() {
            children.push(metadata_tag_empty_category_target(
                category_id.as_str(),
                locked,
                drag_active,
            ));
        } else {
            let pills = group.tags.into_iter().map(|tag| {
                metadata_tag_library_row(
                    tag,
                    category_id.as_str(),
                    locked,
                    selected_tags,
                    drag_active,
                )
            });
            children.push(
                ui::wrap(pills, TAG_LIBRARY_PILL_GAP, TAG_LIBRARY_PILL_GAP)
                    .key(format!("metadata-tag-category-pills-{}", group.id))
                    .fill_width(),
            );
        }
    }
    ui::column(children)
        .key(format!("metadata-tag-category-group-{}", group.id))
        .spacing(2.0)
        .fill_width()
}

fn metadata_tag_category_header(
    category_id: String,
    label: String,
    locked: bool,
    drag_active: bool,
    drop_hover: bool,
) -> ui::View<GuiMessage> {
    let category_for_input = category_id.clone();
    let mut input = ui::interactive_row();
    if drag_active && !locked {
        input = input.droppable(true);
    }
    let input = input
        .mapped(move |message| match message {
            ui::InteractiveRowMessage::Activate => {
                GuiMessage::ToggleMetadataTagCategory(category_for_input.clone())
            }
            ui::InteractiveRowMessage::DoubleActivate
            | ui::InteractiveRowMessage::SecondaryActivate { .. } => GuiMessage::Noop,
            ui::InteractiveRowMessage::Drop => GuiMessage::DropMetadataTagOnCategory {
                category_id: category_for_input.clone(),
            },
            ui::InteractiveRowMessage::HoverDropTarget => {
                GuiMessage::HoverMetadataTagDropCategory {
                    category_id: category_for_input.clone(),
                }
            }
            ui::InteractiveRowMessage::Drag(_) => GuiMessage::Noop,
        })
        .key(format!("metadata-tag-category-hit-{category_id}"))
        .input_only()
        .fill_width()
        .height(22.0);
    let style = ui::WidgetStyle {
        tone: if drop_hover {
            ui::WidgetTone::Warning
        } else {
            ui::WidgetTone::Neutral
        },
        prominence: if drop_hover {
            ui::WidgetProminence::Strong
        } else {
            ui::WidgetProminence::Subtle
        },
    };
    ui::stack([
        ui::row([ui::text(label)
            .key(format!("metadata-tag-category-label-{category_id}"))
            .fill_width()
            .height(22.0)
            .truncate()])
        .style(style)
        .padding_x(4.0)
        .fill_width()
        .height(22.0),
        input,
    ])
    .key(format!("metadata-tag-category-{}", category_id))
    .fill_width()
    .height(22.0)
}

fn metadata_tag_library_row(
    tag: String,
    category_id: &str,
    locked: bool,
    selected_tags: &[String],
    drag_active: bool,
) -> ui::View<GuiMessage> {
    let selected = selected_tags.iter().any(|selected| selected == &tag);
    let style = ui::WidgetStyle {
        tone: metadata_tag_category_tone(category_id),
        prominence: if selected || locked {
            ui::WidgetProminence::Strong
        } else {
            ui::WidgetProminence::Subtle
        },
    };
    let width = metadata_tag_pill_width(&tag);
    let mut visual = ui::badge(tag.clone())
        .message(GuiMessage::Noop)
        .key(format!("metadata-tag-library-pill-visual-{tag}"))
        .style(style)
        .sizing(ui::WidgetSizing::fixed(ui::Vector2::new(
            width,
            TAG_LIBRARY_PILL_HEIGHT,
        )))
        .height(TAG_LIBRARY_PILL_HEIGHT)
        .width(width);
    if !selected && !locked {
        visual = visual.subtle();
    }

    let tag_for_input = tag.clone();
    let category_for_input = category_id.to_string();
    let mut input = ui::interactive_row();
    if !locked {
        input = input.draggable();
        if drag_active {
            input = input.droppable(true);
        }
    }
    let input = input
        .mapped(move |message| match message {
            ui::InteractiveRowMessage::Activate => {
                GuiMessage::ToggleMetadataTag(tag_for_input.clone())
            }
            ui::InteractiveRowMessage::DoubleActivate => GuiMessage::Noop,
            ui::InteractiveRowMessage::SecondaryActivate { position } => {
                GuiMessage::OpenMetadataTagContextMenu {
                    tag: tag_for_input.clone(),
                    position,
                }
            }
            ui::InteractiveRowMessage::Drag(drag) => GuiMessage::DragMetadataTag {
                tag: tag_for_input.clone(),
                drag,
            },
            ui::InteractiveRowMessage::Drop => GuiMessage::DropMetadataTagOnCategory {
                category_id: category_for_input.clone(),
            },
            ui::InteractiveRowMessage::HoverDropTarget => {
                GuiMessage::HoverMetadataTagDropCategory {
                    category_id: category_for_input.clone(),
                }
            }
        })
        .key(format!("metadata-tag-library-row-hit-{tag}"))
        .input_only()
        .width(width)
        .height(TAG_LIBRARY_PILL_HEIGHT);
    ui::stack([visual, input])
        .key(format!("metadata-tag-library-row-{tag}"))
        .width(width)
        .height(TAG_LIBRARY_PILL_HEIGHT)
}

fn metadata_tag_empty_category_target(
    category_id: &str,
    locked: bool,
    drag_active: bool,
) -> ui::View<GuiMessage> {
    let category_for_input = category_id.to_string();
    let mut input = ui::interactive_row();
    if drag_active && !locked {
        input = input.droppable(true);
    }
    let input = input
        .mapped(move |message| match message {
            ui::InteractiveRowMessage::Drop => GuiMessage::DropMetadataTagOnCategory {
                category_id: category_for_input.clone(),
            },
            ui::InteractiveRowMessage::DoubleActivate
            | ui::InteractiveRowMessage::SecondaryActivate { .. } => GuiMessage::Noop,
            ui::InteractiveRowMessage::HoverDropTarget => {
                GuiMessage::HoverMetadataTagDropCategory {
                    category_id: category_for_input.clone(),
                }
            }
            ui::InteractiveRowMessage::Activate | ui::InteractiveRowMessage::Drag(_) => {
                GuiMessage::Noop
            }
        })
        .key(format!("metadata-tag-empty-category-hit-{category_id}"))
        .input_only()
        .fill_width()
        .height(20.0);
    ui::stack([
        ui::text("No tags yet")
            .height(20.0)
            .fill_width()
            .truncate()
            .padding(4.0),
        input,
    ])
    .key(format!("metadata-tag-empty-category-{category_id}"))
    .fill_width()
    .height(20.0)
}

fn metadata_tag_pill_width(tag: &str) -> f32 {
    (tag.chars().count() as f32 * 7.0 + 22.0).clamp(38.0, 180.0)
}

fn metadata_tag_category_tone(category_id: &str) -> ui::WidgetTone {
    match category_id {
        "playback-type" => ui::WidgetTone::Warning,
        "sound-type" => ui::WidgetTone::Accent,
        "character" => ui::WidgetTone::Success,
        "prefix" => ui::WidgetTone::Danger,
        "tuning-scale" => ui::WidgetTone::Neutral,
        _ => ui::WidgetTone::Neutral,
    }
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
