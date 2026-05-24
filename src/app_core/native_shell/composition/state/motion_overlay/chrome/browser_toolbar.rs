use super::*;

impl NativeShellState {
    pub(super) fn render_browser_toolbar_motion(
        &self,
        primitives: &mut impl PrimitiveSink,
        style: &StyleTokens,
        model: &NativeMotionModel,
        motion_wave: f32,
    ) {
        self.render_browser_search_hover(primitives, style, motion_wave);
        self.render_rating_filter_hover(primitives, style, model, motion_wave);
        self.render_playback_age_filter_hover(primitives, style, model, motion_wave);
        self.render_marked_filter_hover(primitives, style, model, motion_wave);
    }

    fn render_browser_search_hover(
        &self,
        primitives: &mut impl PrimitiveSink,
        style: &StyleTokens,
        motion_wave: f32,
    ) {
        let Some(search_field_rect) = self
            .browser_toolbar_layout
            .as_ref()
            .map(|toolbar| toolbar.search_field)
            .filter(|rect| rect.width() > 1.0)
        else {
            return;
        };
        if self.hovered_browser_search_field && self.browser_search_editor_visual.is_none() {
            render_browser_search_field_hover_overlay(
                primitives,
                style,
                style.sizing,
                search_field_rect,
                motion_wave,
            );
        }
    }

    fn render_rating_filter_hover(
        &self,
        primitives: &mut impl PrimitiveSink,
        style: &StyleTokens,
        model: &NativeMotionModel,
        motion_wave: f32,
    ) {
        let Some((chip_rect, rating_level)) =
            self.browser_toolbar_layout.as_ref().and_then(|toolbar| {
                let hovered_level = self.hovered_browser_rating_filter_level?;
                let index = browser_rating_filter_chip_index(hovered_level)?;
                let chip_rect = toolbar.rating_filter_chips[index];
                (chip_rect.width() > 1.0).then_some((chip_rect, hovered_level))
            })
        else {
            return;
        };
        let active = browser_rating_filter_chip_index(rating_level)
            .and_then(|index| model.active_rating_filters.get(index))
            .copied()
            .unwrap_or(false);
        render_browser_rating_filter_chip_hover_overlay(
            primitives,
            style,
            style.sizing,
            chip_rect,
            rating_level,
            active,
            motion_wave,
        );
    }

    fn render_playback_age_filter_hover(
        &self,
        primitives: &mut impl PrimitiveSink,
        style: &StyleTokens,
        model: &NativeMotionModel,
        motion_wave: f32,
    ) {
        let Some((chip_rect, chip)) = self.browser_toolbar_layout.as_ref().and_then(|toolbar| {
            let hovered_chip = self.hovered_browser_playback_age_filter_chip?;
            let index = browser_playback_age_filter_chip_index(hovered_chip)?;
            let chip_rect = toolbar.playback_age_filter_chips[index];
            (chip_rect.width() > 1.0).then_some((chip_rect, hovered_chip))
        }) else {
            return;
        };
        let active = browser_playback_age_filter_chip_index(chip)
            .and_then(|index| model.active_playback_age_filters.get(index))
            .copied()
            .unwrap_or(false);
        render_browser_playback_age_filter_chip_hover_overlay(
            primitives,
            style,
            style.sizing,
            chip_rect,
            chip,
            active,
            motion_wave,
        );
    }

    fn render_marked_filter_hover(
        &self,
        primitives: &mut impl PrimitiveSink,
        style: &StyleTokens,
        model: &NativeMotionModel,
        motion_wave: f32,
    ) {
        let Some(chip_rect) = self
            .browser_toolbar_layout
            .as_ref()
            .map(|toolbar| toolbar.marked_filter_chip)
            .filter(|rect| rect.width() > 1.0)
            .filter(|_| self.hovered_browser_marked_filter)
        else {
            return;
        };
        emit_primitive(
            primitives,
            Primitive::Rect(FillRect {
                rect: chip_rect,
                color: browser_marked_filter_chip_hover_fill(
                    style,
                    model.marked_filter_active,
                    motion_wave,
                ),
            }),
        );
        push_border(
            primitives,
            chip_rect,
            browser_marked_filter_chip_hover_border(style, model.marked_filter_active, motion_wave),
            style.sizing.border_width,
        );
    }
}
