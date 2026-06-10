use radiant::{
    layout::LayoutOutput,
    prelude as ui,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{InteractiveRowWidget, WidgetId},
};

use crate::native_app::app::GuiMessage;

pub(super) const SIDEBAR_ROW_HOVER_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 24,
};
pub(super) const SIDEBAR_ROW_PRESSED_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 108,
    b: 88,
    a: 170,
};
pub(super) const SIDEBAR_ROW_SELECTED_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 34,
};
pub(super) const SIDEBAR_ROW_DROP_TARGET_FILL: ui::Rgba8 = ui::Rgba8 {
    r: 255,
    g: 130,
    b: 78,
    a: 220,
};

pub(super) fn sidebar_row_underlay(content: ui::View<GuiMessage>) -> SidebarRowUnderlayBuilder {
    SidebarRowUnderlayBuilder {
        content,
        row: ui::interactive_row().custom_paint_hit_target(),
        input_id: None,
        selected: false,
        active_target: false,
        leading_marker: None,
    }
}

pub(super) struct SidebarRowUnderlayBuilder {
    content: ui::View<GuiMessage>,
    row: ui::InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    selected: bool,
    active_target: bool,
    leading_marker: Option<ui::DenseRowMarkerStyle>,
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

    pub(super) fn leading_marker_if(
        mut self,
        condition: bool,
        marker: ui::DenseRowMarkerStyle,
    ) -> Self {
        if condition {
            self.leading_marker = Some(marker);
        }
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
            leading_marker,
        } = self;
        let mut input = ui::custom_widget_direct(SidebarRowHitTarget {
            row: row.widget(),
            actions,
            selected,
            active_target,
            leading_marker,
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
    leading_marker: Option<ui::DenseRowMarkerStyle>,
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
        _theme: &ThemeTokens,
    ) {
        let mut chrome = self.row.dense_chrome_parts(
            ui::InteractiveRowVisualStateParts {
                selected: self.selected,
                active_target: self.active_target,
                ..ui::InteractiveRowVisualStateParts::default()
            },
            sidebar_row_palette(self.row.paints_interaction_fill()),
        );
        if let Some(marker) = self.leading_marker {
            chrome = chrome.leading_marker(marker);
        }
        self.row.push_dense_chrome(primitives, bounds, chrome);
    }
}

pub(super) fn sidebar_row_palette(paints_interaction: bool) -> ui::DenseRowPalette {
    ui::DenseRowPalette::new()
        .selected(SIDEBAR_ROW_SELECTED_FILL)
        .interaction_fills_if(
            paints_interaction,
            SIDEBAR_ROW_HOVER_FILL,
            SIDEBAR_ROW_PRESSED_FILL,
        )
        .active_target(SIDEBAR_ROW_DROP_TARGET_FILL)
}
