use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(in crate::gui_runtime::native_vello) fn ui_scale_factor(&self) -> f32 {
        self.window
            .as_ref()
            .map(|window| {
                let scale = window.scale_factor() as f32;
                scale.clamp(1.0, 3.0)
            })
            .unwrap_or(1.0)
    }

    pub(in crate::gui_runtime::native_vello) fn rebuild_layout(
        &mut self,
        invalidation: RuntimeLayoutInvalidation,
    ) -> DirtySegments {
        let Some(surface) = self.render_surface.as_ref() else {
            return DirtySegments::empty();
        };

        let viewport = Vector2::new(surface.config.width as f32, surface.config.height as f32);
        let style = StyleTokens::for_viewport_with_scale(viewport.x, self.ui_scale_factor());
        let existing_layout = self.shell_layout.as_ref().map(Arc::clone);
        let existing_viewport = existing_layout
            .as_ref()
            .map(|layout| Vector2::new(layout.root.rect.width(), layout.root.rect.height()));
        let existing_scale = existing_layout.as_ref().map(|layout| layout.ui_scale);
        let base_style = StyleTokens::for_viewport_width(viewport.x);
        let next_ui_scale = if base_style.sizing.font_title > 0.0 {
            (style.sizing.font_title / base_style.sizing.font_title).clamp(1.0, 3.0)
        } else {
            1.0
        };
        let must_reset_runtime = invalidation.full_rebuild
            || existing_layout.is_none()
            || existing_viewport != Some(viewport)
            || existing_scale != Some(next_ui_scale);

        if must_reset_runtime {
            self.layout_runtime.reset();
        } else {
            for subtree in &invalidation.subtrees {
                self.layout_runtime.mark_subtree_dirty(
                    subtree.tree_kind,
                    subtree.node_id,
                    subtree.dirty_kind,
                );
            }
        }
        self.style_cache = Some(style);
        self.shell_layout = Some(Arc::new(ShellLayout::build_with_style_and_runtime(
            viewport,
            &style,
            &mut self.layout_runtime,
        )));
        if must_reset_runtime {
            self.static_segment_graph.clear();
        }
        if let Some(point) = self.pending_cursor.take() {
            let _ = self.process_cursor_move_immediately(point);
        }
        invalidation.dirty_segments
    }

    /// Borrow the retained shell layout while mutating runtime state without
    /// cloning the full layout payload.
    pub(in crate::gui_runtime::native_vello) fn with_shell_layout<T>(
        &mut self,
        work: impl FnOnce(&mut Self, &ShellLayout) -> T,
    ) -> Option<T> {
        let layout = self.shell_layout.take()?;
        let result = work(self, layout.as_ref());
        self.shell_layout = Some(layout);
        Some(result)
    }

    pub(in crate::gui_runtime::native_vello) fn request_redraw_if_needed(&mut self) {
        if self.redraw_requested {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
            self.redraw_requested = true;
        }
    }

    pub(in crate::gui_runtime::native_vello) fn build_style_for_layout(
        layout: &ShellLayout,
    ) -> StyleTokens {
        StyleTokens::for_viewport_with_scale(layout.root.rect.width(), layout.ui_scale)
    }

    pub(in crate::gui_runtime::native_vello) fn cached_style_for_layout(
        &self,
        layout: &ShellLayout,
    ) -> StyleTokens {
        self.style_cache
            .unwrap_or_else(|| Self::build_style_for_layout(layout))
    }
}
