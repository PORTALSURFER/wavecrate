use radiant::{
    layout::LayoutOutput,
    prelude as ui,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{InteractiveRowWidget, WidgetId},
};

use crate::native_app::app::GuiMessage;

pub(super) const SIDEBAR_ROW_STYLE: ui::WidgetStyle =
    ui::WidgetStyle::subtle(ui::WidgetTone::Accent);

pub(super) fn sidebar_row_underlay(content: ui::View<GuiMessage>) -> SidebarRowUnderlayBuilder {
    SidebarRowUnderlayBuilder {
        content,
        row: ui::interactive_row().custom_paint_hit_target(),
        input_id: None,
        selected: false,
        active_target: false,
    }
}

pub(super) struct SidebarRowUnderlayBuilder {
    content: ui::View<GuiMessage>,
    row: ui::InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    selected: bool,
    active_target: bool,
}

impl SidebarRowUnderlayBuilder {
    pub(super) fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(ui::stable_widget_id(scope, key));
        self
    }

    pub(super) fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(ui::stable_widget_id_u64(scope, key));
        self
    }

    pub(super) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(super) fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.row = self.row.tracked_drop_target(drag_active, active_target);
        self.active_target = active_target;
        self
    }

    pub(super) fn actions(
        self,
        actions: ui::InteractiveRowActions<GuiMessage>,
    ) -> ui::View<GuiMessage> {
        let Self {
            content,
            row,
            input_id,
            selected,
            active_target,
        } = self;
        let mut input = ui::custom_widget_direct(SidebarRowHitTarget {
            row: row.widget(),
            actions,
            selected,
            active_target,
        });
        if let Some(input_id) = input_id {
            input = input.id(input_id);
        }
        ui::input_underlay(content, input)
    }
}

#[derive(Clone)]
struct SidebarRowHitTarget {
    row: InteractiveRowWidget,
    actions: ui::InteractiveRowActions<GuiMessage>,
    selected: bool,
    active_target: bool,
}

impl ui::EmbeddedInteractiveRowWidget for SidebarRowHitTarget {
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
        bounds: ui::Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let chrome = self.row.dense_chrome_parts(
            ui::InteractiveRowVisualStateParts {
                selected: self.selected,
                active_target: self.active_target,
                ..ui::InteractiveRowVisualStateParts::default()
            },
            sidebar_row_palette(theme, self.row.paints_interaction_fill()),
        );
        self.row.push_dense_chrome(primitives, bounds, chrome);
    }
}

pub(super) fn sidebar_row_palette(
    theme: &ThemeTokens,
    paints_interaction: bool,
) -> ui::DenseRowPalette {
    let palette = ui::dense_row_palette_from_style(theme, SIDEBAR_ROW_STYLE);
    if paints_interaction {
        palette
    } else {
        ui::DenseRowPalette::new()
            .selected(palette.selected.expect("dense-row selected fill"))
            .active_target(palette.active_target.expect("dense-row active-target fill"))
    }
}

#[cfg(test)]
pub(super) fn sidebar_row_palette_for_tests() -> ui::DenseRowPalette {
    ui::dense_row_palette_from_style(&ThemeTokens::default(), SIDEBAR_ROW_STYLE)
}

#[cfg(test)]
pub(super) fn sidebar_row_hover_fill_for_tests() -> ui::Rgba8 {
    sidebar_row_palette_for_tests()
        .hovered
        .expect("dense-row hover fill")
}

#[cfg(test)]
pub(super) fn sidebar_row_selected_fill_for_tests() -> ui::Rgba8 {
    sidebar_row_palette_for_tests()
        .selected
        .expect("dense-row selected fill")
}

#[cfg(test)]
pub(super) fn sidebar_row_active_target_fill_for_tests() -> ui::Rgba8 {
    sidebar_row_palette_for_tests()
        .active_target
        .expect("dense-row active-target fill")
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::widgets::{Widget, WidgetInput};

    fn hit_target(selected: bool, active_target: bool) -> SidebarRowHitTarget {
        SidebarRowHitTarget {
            row: ui::interactive_row().custom_paint_hit_target().widget(),
            actions: ui::row_actions(),
            selected,
            active_target,
        }
    }

    #[test]
    fn hovered_sidebar_row_paints_neutral_sample_list_hover_fill() {
        let bounds = ui::Rect::from_size(120.0, 22.0);
        let mut target = hit_target(false, false);
        target.handle_input(bounds, WidgetInput::pointer_move(ui::Point::new(8.0, 8.0)));

        let plan = target.paint_plan_with_defaults(bounds);

        assert!(
            plan.fill_rects()
                .any(|fill| fill.color == sidebar_row_hover_fill_for_tests()),
            "hovered sidebar rows should use the neutral sample-list hover fill"
        );
    }

    #[test]
    fn hovered_selected_sidebar_row_keeps_neutral_hover_priority() {
        let bounds = ui::Rect::from_size(120.0, 22.0);
        let mut target = hit_target(true, false);
        target.handle_input(bounds, WidgetInput::pointer_move(ui::Point::new(8.0, 8.0)));

        let plan = target.paint_plan_with_defaults(bounds);

        assert!(
            plan.fill_rects()
                .any(|fill| fill.color == sidebar_row_hover_fill_for_tests()),
            "hover should stay neutral even when a sidebar row is selected"
        );
        assert!(
            !plan
                .fill_rects()
                .any(|fill| fill.color == sidebar_row_selected_fill_for_tests()),
            "hover fill has priority over the selected fill"
        );
    }

    #[test]
    fn selected_sidebar_row_paints_orange_highlight_fill() {
        let bounds = ui::Rect::from_size(120.0, 22.0);
        let target = hit_target(true, false);

        let plan = target.paint_plan_with_defaults(bounds);

        assert!(
            plan.fill_rects()
                .any(|fill| fill.color == sidebar_row_selected_fill_for_tests()),
            "selected sidebar rows should use the orange selected/focused highlight"
        );
    }
}
