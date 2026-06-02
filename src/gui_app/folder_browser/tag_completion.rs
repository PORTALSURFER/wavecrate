use radiant::prelude as ui;

use crate::gui_app::metadata_tags::MetadataTagCompletionOption;

use super::GuiMessage;

const MAX_TAG_COMPLETION_ROWS: usize = 6;
const TAG_COMPLETION_ROW_HEIGHT: f32 = 18.0;
const TAG_COMPLETION_POPUP_VERTICAL_CHROME: f32 = 6.0;

pub(super) fn tag_completion_panel_layer(
    options: &[MetadataTagCompletionOption],
    content_width: f32,
    content_height: f32,
    tag_field_height: f32,
) -> ui::View<GuiMessage> {
    if options.is_empty() {
        return ui::empty().fill_width();
    }
    let trigger_y = content_height - tag_field_height;
    ui::compact_option_list_floating_above(ui::CompactOptionListFloatingAboveParts::new(
        tag_completion_options(options, content_width),
        0.0,
        trigger_y,
        3.0,
        content_width,
    ))
    .key("metadata-tag-completion-panel-layer")
    .fill()
}

fn tag_completion_options(
    options: &[MetadataTagCompletionOption],
    content_width: f32,
) -> ui::CompactOptionListParts {
    let tag_width = (content_width * 0.48).clamp(70.0, 140.0);
    let items = options
        .iter()
        .map(|option| {
            ui::CompactOptionListItem::new(option.tag.clone())
                .secondary_label(option.category)
                .selected(option.selected)
        })
        .collect::<Vec<_>>();
    ui::CompactOptionListParts::new(items, tag_width)
        .max_visible_rows(MAX_TAG_COMPLETION_ROWS)
        .row_height(TAG_COMPLETION_ROW_HEIGHT)
        .vertical_chrome(TAG_COMPLETION_POPUP_VERTICAL_CHROME)
}
