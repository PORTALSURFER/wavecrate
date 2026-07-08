use radiant::prelude as ui;
#[cfg(test)]
use radiant::theme::ThemeTokens;

use super::identity;
use crate::native_app::app::GuiMessage;

const SAMPLE_ROW_STYLE: ui::WidgetStyle = ui::WidgetStyle::subtle(ui::WidgetTone::Accent);
const COPY_FLASH_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 71,
    g: 220,
    b: 255,
    a: 118,
};
const PROTECTED_SOURCE_ERROR_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 69,
    b: 54,
    a: 155,
};
const PROTECTED_SOURCE_ERROR_HOVER_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 82,
    b: 62,
    a: 185,
};
const CUT_PENDING_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 182,
    g: 111,
    b: 44,
    a: 120,
};
const CUT_PENDING_HOVER_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 214,
    g: 132,
    b: 52,
    a: 150,
};
const CUT_PENDING_MARKER: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 163,
    b: 73,
    a: 245,
};
const MISSING_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 130,
    g: 30,
    b: 28,
    a: 145,
};
const MISSING_HOVER_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 170,
    g: 42,
    b: 38,
    a: 175,
};
const MISSING_MARKER: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 69,
    b: 54,
    a: 250,
};
const SELECTED_MARKER: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 82,
    b: 62,
    a: 245,
};
const FOCUSED_OUTLINE: ui::Rgba8 = ui::Rgba8 {
    r: 80,
    g: 190,
    b: 255,
    a: 235,
};
const FOCUSED_OUTLINE_INSET: f32 = 0.5;
const FOCUSED_OUTLINE_WIDTH: f32 = 1.5;
const CACHED_MARKER: ui::Rgba8 = ui::Rgba8 {
    r: 226,
    g: 226,
    b: 226,
    a: 210,
};

pub(in crate::native_app) struct SampleFileHitTargetModel<'a> {
    pub(in crate::native_app) file_id: &'a str,
    pub(in crate::native_app) explicitly_selected: bool,
    pub(in crate::native_app) focused: bool,
    pub(in crate::native_app) copy_flash: bool,
    pub(in crate::native_app) protected_source_error_flash: bool,
    pub(in crate::native_app) cut_pending: bool,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) drag_source: bool,
    pub(in crate::native_app) cached: bool,
    pub(in crate::native_app) missing: bool,
    pub(in crate::native_app) hit_path: String,
    pub(in crate::native_app) help_tooltips_enabled: bool,
}

pub(super) fn sample_file_hit_target(
    content: ui::View<GuiMessage>,
    model: SampleFileHitTargetModel<'_>,
) -> ui::View<GuiMessage> {
    let actions = sample_file_row_actions(model.hit_path.clone());
    sample_file_hit_target_builder(content, &model)
        .stable_row_identity(
            identity::RETAINED_SAMPLE_ROW_INPUT_SCOPE,
            identity::retained_sample_row_key(model.file_id),
        )
        .actions(actions)
        .tooltip_if(
            model.help_tooltips_enabled,
            "Sample row: select, double-click to load, drag to copy, right-click for actions.",
        )
}

fn sample_file_hit_target_builder(
    content: ui::View<GuiMessage>,
    model: &SampleFileHitTargetModel<'_>,
) -> ui::InteractiveRowUnderlayBuilder<GuiMessage> {
    let visual_state = ui::InteractiveRowVisualStateParts {
        selected: model.explicitly_selected
            || model.copy_flash
            || model.protected_source_error_flash
            || model.cut_pending
            || model.missing,
        ..ui::InteractiveRowVisualStateParts::default()
    };
    let underlay = ui::interactive_row_underlay(content)
        .tracked_drag_source(model.drag_active, model.drag_source)
        .activation_modifiers()
        .custom_paint_hit_target()
        .style(SAMPLE_ROW_STYLE)
        .visual_state(visual_state)
        .leading_marker_if(
            model.explicitly_selected || model.cut_pending || model.missing,
            ui::DenseRowMarkerStyle::new(
                ui::DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
                if model.missing {
                    MISSING_MARKER
                } else if model.cut_pending {
                    CUT_PENDING_MARKER
                } else {
                    SELECTED_MARKER
                },
            ),
        )
        .trailing_marker_if(
            model.cached && !model.explicitly_selected && !model.copy_flash && !model.cut_pending,
            ui::DenseRowMarkerStyle::new(ui::DenseRowMarkerParts::trailing(2.0), CACHED_MARKER),
        )
        .outline_if(model.focused, sample_file_focus_outline());
    if let Some(palette) = sample_file_row_palette(model) {
        underlay.dense_chrome_palette(palette)
    } else {
        underlay
    }
}

fn sample_file_row_actions(path: String) -> ui::InteractiveRowActions<GuiMessage> {
    ui::row_actions()
        .primary_with_modifiers_key(path.clone(), |path, modifiers| {
            GuiMessage::SelectSampleWithModifiers { path, modifiers }
        })
        .double_key(path.clone(), |path| GuiMessage::SelectSampleWithModifiers {
            path,
            modifiers: Default::default(),
        })
        .secondary_key(path.clone(), |path, position| {
            GuiMessage::OpenSampleContextMenu { path, position }
        })
        .drag_key(path, |path, drag| GuiMessage::DragSampleFile { path, drag })
}

fn sample_file_row_palette(model: &SampleFileHitTargetModel<'_>) -> Option<ui::DenseRowPalette> {
    if model.copy_flash {
        return Some(
            ui::DenseRowPalette::new()
                .selected(COPY_FLASH_FILL)
                .selected_hovered(COPY_FLASH_FILL)
                .interaction_fills(COPY_FLASH_FILL, COPY_FLASH_FILL),
        );
    }
    if model.protected_source_error_flash {
        return Some(
            ui::DenseRowPalette::new()
                .selected(PROTECTED_SOURCE_ERROR_FILL)
                .selected_hovered(PROTECTED_SOURCE_ERROR_HOVER_FILL)
                .interaction_fills(
                    PROTECTED_SOURCE_ERROR_HOVER_FILL,
                    PROTECTED_SOURCE_ERROR_HOVER_FILL,
                ),
        );
    }
    if model.missing {
        return Some(
            ui::DenseRowPalette::new()
                .selected(MISSING_FILL)
                .selected_hovered(MISSING_HOVER_FILL)
                .interaction_fills(MISSING_HOVER_FILL, MISSING_HOVER_FILL),
        );
    }
    if model.cut_pending {
        return Some(
            ui::DenseRowPalette::new()
                .selected(CUT_PENDING_FILL)
                .selected_hovered(CUT_PENDING_HOVER_FILL)
                .interaction_fills(CUT_PENDING_HOVER_FILL, CUT_PENDING_HOVER_FILL),
        );
    }
    None
}

fn sample_file_focus_outline() -> ui::DenseRowOutlineStyle {
    ui::DenseRowOutlineStyle::new(
        FOCUSED_OUTLINE_INSET,
        FOCUSED_OUTLINE,
        FOCUSED_OUTLINE_WIDTH,
    )
}

#[cfg(test)]
pub(in crate::native_app) fn sample_file_hit_target_for_tests(
    content: ui::View<GuiMessage>,
    model: SampleFileHitTargetModel<'_>,
    input_id: u64,
) -> ui::View<GuiMessage> {
    let actions = sample_file_row_actions(model.hit_path.clone());
    sample_file_hit_target_builder(content, &model)
        .input_id(input_id)
        .actions(actions)
}

#[cfg(test)]
fn sample_row_palette_for_tests() -> ui::DenseRowPalette {
    ui::dense_row_palette_from_style(&ThemeTokens::default(), SAMPLE_ROW_STYLE)
}

#[cfg(test)]
#[path = "hit_target_tests.rs"]
mod tests;
