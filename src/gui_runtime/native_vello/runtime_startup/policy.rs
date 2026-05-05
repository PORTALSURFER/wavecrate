use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Keep the native window hidden until startup sequencing decides reveal timing.
    pub(in crate::gui_runtime::native_vello) fn startup_should_launch_hidden() -> bool {
        true
    }

    /// Use a placeholder-first startup path by default so the window can reveal
    /// after a lightweight first scene while the full model refresh continues.
    pub(in crate::gui_runtime::native_vello) fn startup_should_defer_first_model_pull() -> bool {
        true
    }

    /// Resolve a deterministic startup clear color used before style/layout are ready.
    pub(in crate::gui_runtime::native_vello) fn startup_placeholder_clear_color() -> Rgba8 {
        StyleTokens::for_viewport_width(1280.0).clear_color
    }

    pub(in crate::gui_runtime::native_vello) fn build_window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_maximized(self.options.maximized)
            .with_decorations(self.options.decorations)
            .with_visible(!Self::startup_should_launch_hidden());
        if let Some([w, h]) = self.options.inner_size {
            attrs = attrs.with_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }
        if let Some([w, h]) = self.options.min_inner_size {
            attrs = attrs.with_min_inner_size(Size::Logical(LogicalSize::new(w as f64, h as f64)));
        }
        if let Some(icon) = self.options.icon.as_ref().and_then(icon_from_rgba) {
            attrs = attrs.with_window_icon(Some(icon));
        }
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::WindowAttributesExtWindows;
            attrs = attrs.with_drag_and_drop(true);
        }
        attrs
    }

    /// Arm the hidden-startup reveal timeout so redraw stalls cannot deadlock launch.
    pub(in crate::gui_runtime::native_vello) fn arm_startup_reveal_deadline(
        &mut self,
        now: Instant,
    ) {
        if Self::startup_should_launch_hidden() && !self.startup_window_visible {
            self.startup_reveal_deadline = Some(now + STARTUP_REVEAL_STALL_TIMEOUT);
        }
    }
}
