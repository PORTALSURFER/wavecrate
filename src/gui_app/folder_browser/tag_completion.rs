use radiant::{
    gui::types::Rect,
    layout::LayoutOutput,
    prelude as ui,
    runtime::{PaintFillRect, PaintPrimitive, PaintText, PaintTextAlign, PaintTextRun},
    theme::ThemeTokens,
    widgets::{
        TextWrap, Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing, WidgetStyle,
        WidgetTone,
    },
};

use crate::gui_app::metadata_tags::MetadataTagCompletionOption;

use super::GuiMessage;

const MAX_TAG_COMPLETION_ROWS: usize = 6;
const TAG_COMPLETION_ROW_HEIGHT: f32 = 18.0;
const TAG_COMPLETION_POPUP_VERTICAL_CHROME: f32 = 6.0;
const TAG_FIELD_CONTROL_HEIGHT: f32 = 18.0;

#[derive(Clone, Debug)]
pub(super) struct TagCompletionGhost {
    common: WidgetCommon,
    suffix: String,
}

impl TagCompletionGhost {
    pub(super) fn new(suffix: String, width: f32) -> Self {
        Self {
            common: WidgetCommon::new(
                0,
                WidgetSizing::fixed(ui::Vector2::new(
                    width.max(1.0),
                    TAG_FIELD_CONTROL_HEIGHT - 3.0,
                )),
            ),
            suffix,
        }
    }
}

impl Widget for TagCompletionGhost {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn needs_state_synchronization(&self) -> bool {
        false
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let fill = theme.accent_mint.blend_toward(theme.bg_primary, 0.12);
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: fill,
        }));
        let text_rect = Rect::from_min_max(
            ui::Point::new(bounds.min.x + 3.0, bounds.min.y),
            ui::Point::new(bounds.max.x - 3.0, bounds.max.y),
        );
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: PaintText::from(self.suffix.clone()),
            rect: text_rect,
            font_size: 13.0,
            baseline: Some((text_rect.height() * 0.5 + 13.0 * 0.35).max(0.0)),
            color: theme.bg_primary,
            align: PaintTextAlign::Left,
            wrap: TextWrap::None,
        }));
    }
}

fn tag_completion_popup_height(options: &[MetadataTagCompletionOption]) -> f32 {
    if options.is_empty() {
        return 0.0;
    }
    let rows = options.len().min(MAX_TAG_COMPLETION_ROWS);
    rows as f32 * TAG_COMPLETION_ROW_HEIGHT + TAG_COMPLETION_POPUP_VERTICAL_CHROME
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
    let popup_y = content_height - tag_field_height - 3.0 - popup_height;
    ui::floating_layer(
        ui::Point::new(0.0, popup_y),
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
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: ui::WidgetProminence::Subtle,
        })
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
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: ui::WidgetProminence::Strong,
        }
    } else {
        WidgetStyle::default()
    })
    .height(TAG_COMPLETION_ROW_HEIGHT)
    .fill_width()
    .spacing(6.0)
}
