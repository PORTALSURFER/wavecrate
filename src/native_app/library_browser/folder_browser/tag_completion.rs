use radiant::prelude as ui;

use crate::native_app::metadata::MetadataTagCompletionOption;

use super::GuiMessage;

const MAX_TAG_COMPLETION_ROWS: usize = 6;
const TAG_COMPLETION_ROW_HEIGHT: f32 = 18.0;
const TAG_COMPLETION_POPUP_VERTICAL_CHROME: f32 = 6.0;

pub(in crate::native_app) const TAG_COMPLETION_POPUP_GAP: f32 = 3.0;

pub(in crate::native_app) fn tag_completion_overlay(
    options: &[MetadataTagCompletionOption],
    content_width: f32,
    inset_x: f32,
    inset_y: f32,
) -> ui::View<GuiMessage> {
    if options.is_empty() {
        return ui::empty().fill_width();
    }
    let parts = tag_completion_options(options, content_width);
    ui::compact_option_list_anchored(ui::CompactOptionListAnchoredParts::new(
        parts,
        content_width,
        ui::LayerHorizontalAnchor::Start,
        ui::LayerVerticalAnchor::End,
        inset_x,
        inset_y,
    ))
    .key("metadata-tag-completion-overlay")
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
