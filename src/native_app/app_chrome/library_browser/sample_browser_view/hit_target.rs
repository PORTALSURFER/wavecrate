use radiant::gui::types::{Rect, Rgba8};
use radiant::layout::LayoutOutput;
use radiant::prelude as ui;
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::{InteractiveRowWidget, PointerModifiers};

use crate::native_app::app::GuiMessage;

const SAMPLE_ROW_STYLE: ui::WidgetStyle = ui::WidgetStyle::subtle(ui::WidgetTone::Accent);
const COPY_FLASH_FILL: Rgba8 = Rgba8 {
    r: 71,
    g: 220,
    b: 255,
    a: 118,
};
const MISSING_FILL: Rgba8 = Rgba8 {
    r: 130,
    g: 30,
    b: 28,
    a: 145,
};
const MISSING_HOVER_FILL: Rgba8 = Rgba8 {
    r: 170,
    g: 42,
    b: 38,
    a: 175,
};
const MISSING_MARKER: Rgba8 = Rgba8 {
    r: 255,
    g: 69,
    b: 54,
    a: 250,
};
const SELECTED_MARKER: Rgba8 = Rgba8 {
    r: 255,
    g: 82,
    b: 62,
    a: 245,
};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SampleFileHitTarget {
    row: InteractiveRowWidget,
    actions: ui::InteractiveRowActions<GuiMessage>,
    selected: bool,
    copy_flash: bool,
    cached: bool,
    missing: bool,
}

impl SampleFileHitTarget {
    pub(in crate::native_app) fn new(
        path: String,
        selected: bool,
        copy_flash: bool,
        drag_active: bool,
        drag_source: bool,
        cached: bool,
        missing: bool,
    ) -> Self {
        let row = ui::interactive_row()
            .tracked_drag_source(drag_active, drag_source)
            .activation_modifiers()
            .custom_paint_hit_target()
            .widget();
        let actions = ui::row_actions()
            .primary_with_modifiers_key(path.clone(), |path, modifiers| {
                GuiMessage::SelectSampleWithModifiers { path, modifiers }
            })
            .double_key(path.clone(), |path| GuiMessage::SelectSampleWithModifiers {
                path,
                modifiers: PointerModifiers::default(),
            })
            .secondary_key(path.clone(), |path, position| {
                GuiMessage::OpenSampleContextMenu { path, position }
            })
            .drag_key(path, |path, drag| GuiMessage::DragSampleFile { path, drag });
        Self {
            row,
            actions,
            selected,
            copy_flash,
            cached,
            missing,
        }
    }
}

impl ui::EmbeddedInteractiveRowWidget for SampleFileHitTarget {
    type Message = GuiMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn interactive_row_actions(&self) -> Option<&ui::InteractiveRowActions<Self::Message>> {
        Some(&self.actions)
    }

    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.row
            .push_dense_chrome(primitives, bounds, self.chrome_parts(theme));
    }
}

impl SampleFileHitTarget {
    fn chrome_parts(&self, theme: &ThemeTokens) -> ui::DenseRowChromeParts {
        self.row
            .dense_chrome_parts(
                ui::InteractiveRowVisualStateParts {
                    selected: self.selected || self.copy_flash || self.missing,
                    ..ui::InteractiveRowVisualStateParts::default()
                },
                self.chrome_palette(theme),
            )
            .trailing_marker_if(
                self.cached && !self.selected && !self.copy_flash,
                ui::DenseRowMarkerStyle::new(
                    ui::DenseRowMarkerParts::trailing(2.0),
                    Rgba8 {
                        r: 226,
                        g: 226,
                        b: 226,
                        a: 210,
                    },
                ),
            )
            .leading_marker_if(
                self.selected || self.missing,
                ui::DenseRowMarkerStyle::new(
                    ui::DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
                    if self.missing {
                        MISSING_MARKER
                    } else {
                        SELECTED_MARKER
                    },
                ),
            )
    }

    fn chrome_palette(&self, theme: &ThemeTokens) -> ui::DenseRowPalette {
        if self.copy_flash {
            return ui::DenseRowPalette::new()
                .selected(COPY_FLASH_FILL)
                .selected_hovered(COPY_FLASH_FILL)
                .interaction_fills(COPY_FLASH_FILL, COPY_FLASH_FILL);
        }
        if self.missing {
            return ui::DenseRowPalette::new()
                .selected(MISSING_FILL)
                .selected_hovered(MISSING_HOVER_FILL)
                .interaction_fills(MISSING_HOVER_FILL, MISSING_HOVER_FILL);
        }
        let palette = ui::dense_row_palette_from_style(theme, SAMPLE_ROW_STYLE);
        if self.row.paints_interaction_fill() {
            palette
        } else {
            ui::DenseRowPalette::new().selected(palette.selected.expect("dense-row selected fill"))
        }
    }
}

#[cfg(test)]
fn sample_row_palette_for_tests() -> ui::DenseRowPalette {
    ui::dense_row_palette_from_style(&ThemeTokens::default(), SAMPLE_ROW_STYLE)
}

#[cfg(test)]
#[path = "hit_target_tests.rs"]
mod tests;
