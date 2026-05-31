use radiant::{
    prelude as ui,
    widgets::{WidgetStyle, WidgetTone},
};

use crate::gui_app::metadata_tags::MetadataTagCompletionOption;

use super::GuiMessage;

const MAX_TAG_COMPLETION_ROWS: usize = 6;
const TAG_COMPLETION_ROW_HEIGHT: f32 = 18.0;
const TAG_COMPLETION_POPUP_VERTICAL_CHROME: f32 = 6.0;

fn tag_completion_popup_height(options: &[MetadataTagCompletionOption]) -> f32 {
    ui::bounded_list_height(
        options.len(),
        MAX_TAG_COMPLETION_ROWS,
        TAG_COMPLETION_ROW_HEIGHT,
        TAG_COMPLETION_POPUP_VERTICAL_CHROME,
    )
}

pub(super) fn tag_completion_panel_layer(
    options: &[MetadataTagCompletionOption],
    content_width: f32,
    content_height: f32,
    tag_field_height: f32,
) -> ui::View<GuiMessage> {
    if options.is_empty() {
        return ui::spacer().height(0.0).fill_width();
    }
    let popup_height = tag_completion_popup_height(options);
    let trigger_y = content_height - tag_field_height;
    ui::floating_layer_above(
        0.0,
        trigger_y,
        3.0,
        ui::Vector2::new(content_width, popup_height),
        tag_completion_popup(options, content_width)
            .key("metadata-tag-completion-popup")
            .fill_width()
            .height(popup_height),
    )
    .key("metadata-tag-completion-panel-layer")
    .fill()
}

fn tag_completion_popup(
    options: &[MetadataTagCompletionOption],
    content_width: f32,
) -> ui::View<GuiMessage> {
    if options.is_empty() {
        return ui::spacer().height(0.0).fill_width();
    }
    let tag_width = (content_width * 0.48).clamp(70.0, 140.0);
    let rows = options
        .iter()
        .take(MAX_TAG_COMPLETION_ROWS)
        .map(|option| tag_completion_row(option, tag_width))
        .collect::<Vec<_>>();
    ui::scroll(ui::column(rows).spacing(0.0).fill_width())
        .style(WidgetStyle::new(
            WidgetTone::Neutral,
            ui::WidgetProminence::Subtle,
        ))
        .padding(3.0)
        .fill_width()
        .height(tag_completion_popup_height(options))
}

fn tag_completion_row(
    option: &MetadataTagCompletionOption,
    tag_width: f32,
) -> ui::View<GuiMessage> {
    ui::row([
        ui::text(option.tag.clone())
            .height(TAG_COMPLETION_ROW_HEIGHT)
            .width(tag_width)
            .truncate(),
        ui::text(option.category.to_string())
            .height(TAG_COMPLETION_ROW_HEIGHT)
            .fill_width()
            .truncate(),
    ])
    .key(format!("metadata-tag-completion-row-{}", option.tag))
    .style(if option.selected {
        WidgetStyle::new(WidgetTone::Accent, ui::WidgetProminence::Strong)
    } else {
        WidgetStyle::default()
    })
    .height(TAG_COMPLETION_ROW_HEIGHT)
    .fill_width()
    .spacing(6.0)
}
