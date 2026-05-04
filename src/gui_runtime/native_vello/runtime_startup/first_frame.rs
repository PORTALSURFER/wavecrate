use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Keep startup first-frame work minimal when the deferred fallback path is armed.
    ///
    /// This preserves static scene rebuild work (for deterministic first paint)
    /// while skipping model and overlay pulls until first present completes.
    pub(in crate::gui_runtime::native_vello) fn prepare_startup_first_frame_scene(&mut self) {
        let _ = self.frame_state.take_model();
        let _ = self.frame_state.take_state_overlay();
        let _ = self.frame_state.take_motion_overlay();
    }

    /// Build one minimal host-titled startup scene for deferred-startup fallback.
    pub(in crate::gui_runtime::native_vello) fn build_startup_placeholder_scene(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
    ) {
        let root = layout.root.rect;
        let panel_width = (root.width() * 0.36).clamp(220.0, 420.0);
        let panel_height = (style.sizing.font_header * 2.8).clamp(58.0, 86.0);
        let panel_min = Point::new(
            root.min.x + (root.width() - panel_width) * 0.5,
            root.min.y + (root.height() - panel_height) * 0.5,
        );
        let panel = UiRect::from_min_size(panel_min, Vector2::new(panel_width, panel_height));
        let accent_height = (panel_height * 0.08).clamp(3.0, 6.0);
        let accent = UiRect::from_min_max(
            panel.min,
            Point::new(panel.max.x, panel.min.y + accent_height),
        );
        let title_text = if self.options.title.trim().is_empty() {
            String::from(crate::gui_runtime::DEFAULT_NATIVE_WINDOW_TITLE)
        } else {
            self.options.title.clone()
        };
        let title = TextRun {
            text: title_text,
            position: Point::new(panel.min.x + 12.0, panel.min.y + 10.0),
            font_size: style.sizing.font_header.max(12.0),
            color: style.text_primary,
            max_width: Some((panel.width() - 24.0).max(20.0)),
            align: TextAlign::Center,
        };
        let subtitle = TextRun {
            text: String::from("Starting interface..."),
            position: Point::new(panel.min.x + 12.0, panel.min.y + panel_height * 0.48),
            font_size: style.sizing.font_meta.max(10.0),
            color: style.text_muted,
            max_width: Some((panel.width() - 24.0).max(20.0)),
            align: TextAlign::Center,
        };

        self.frame_cache.clear_color = style.clear_color;
        self.frame_cache.primitives.clear();
        self.frame_cache.text_runs.clear();
        self.frame_cache.text_runs.push(title.clone());
        self.frame_cache.text_runs.push(subtitle.clone());
        self.hover_overlay_frame_cache.clear_color = style.clear_color;
        self.hover_overlay_frame_cache.primitives.clear();
        self.hover_overlay_frame_cache.text_runs.clear();
        self.focus_overlay_frame_cache.clear_color = style.clear_color;
        self.focus_overlay_frame_cache.primitives.clear();
        self.focus_overlay_frame_cache.text_runs.clear();
        self.modal_overlay_frame_cache.clear_color = style.clear_color;
        self.modal_overlay_frame_cache.primitives.clear();
        self.modal_overlay_frame_cache.text_runs.clear();
        self.waveform_motion_overlay_frame_cache.clear_color = style.clear_color;
        self.waveform_motion_overlay_frame_cache.primitives.clear();
        self.waveform_motion_overlay_frame_cache.text_runs.clear();
        self.chrome_motion_overlay_frame_cache.clear_color = style.clear_color;
        self.chrome_motion_overlay_frame_cache.primitives.clear();
        self.chrome_motion_overlay_frame_cache.text_runs.clear();
        self.clear_color = style.clear_color;

        self.static_scene.reset();
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.surface_base),
            None,
            &to_kurbo_rect(root),
        );
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.surface_raised),
            None,
            &to_kurbo_rect(panel),
        );
        self.static_scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            color_from_rgba(style.accent_mint),
            None,
            &to_kurbo_rect(accent),
        );
        self.text_renderer
            .draw_text_runs(&mut self.static_scene, &[title, subtitle]);
        self.hover_overlay_scene.reset();
        self.focus_overlay_scene.reset();
        self.modal_overlay_scene.reset();
        self.state_overlay_scene.reset();
        self.waveform_motion_overlay_scene.reset();
        self.chrome_motion_overlay_scene.reset();
        self.motion_overlay_scene.reset();
        self.scene.reset();
        self.scene.append(&self.static_scene, None);
    }
}
