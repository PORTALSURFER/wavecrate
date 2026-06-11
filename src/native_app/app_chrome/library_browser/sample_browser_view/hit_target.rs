use radiant::gui::types::{Rect, Rgba8};
use radiant::layout::LayoutOutput;
use radiant::prelude as ui;
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::{InteractiveRowWidget, PointerModifiers};

use crate::native_app::app::GuiMessage;

const SAMPLE_ROW_STYLE: ui::WidgetStyle = ui::WidgetStyle::subtle(ui::WidgetTone::Accent);

#[derive(Clone, Debug)]
pub(in crate::native_app) struct SampleFileHitTarget {
    row: InteractiveRowWidget,
    actions: ui::InteractiveRowActions<GuiMessage>,
    selected: bool,
    cached: bool,
}

impl SampleFileHitTarget {
    pub(in crate::native_app) fn new(
        path: String,
        selected: bool,
        drag_active: bool,
        drag_source: bool,
        cached: bool,
    ) -> Self {
        let row = ui::interactive_row()
            .tracked_drag_source(drag_active, drag_source)
            .clear_hover_on_sync()
            .activation_modifiers()
            .custom_paint_hit_target()
            .widget();
        let actions = ui::InteractiveRowActions::new()
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
            cached,
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
                    selected: self.selected,
                    ..ui::InteractiveRowVisualStateParts::default()
                },
                self.chrome_palette(theme),
            )
            .trailing_marker_if(
                self.cached && !self.selected,
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
                self.selected,
                ui::DenseRowMarkerStyle::new(
                    ui::DenseRowMarkerParts::leading(3.0).vertical_inset(4.0),
                    Rgba8 {
                        r: 255,
                        g: 82,
                        b: 62,
                        a: 245,
                    },
                ),
            )
    }

    fn chrome_palette(&self, theme: &ThemeTokens) -> ui::DenseRowPalette {
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
