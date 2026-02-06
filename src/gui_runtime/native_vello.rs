//! Native `winit + vello` runtime preview used for backend selection rollout.

use super::egui_wgpu::{EguiRunOptions, WindowIconRgba};
use std::sync::Arc;
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Circle, Rect},
    peniko::{Color, Fill},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Icon, Window, WindowAttributes, WindowId},
};

struct NativeVelloRunner {
    options: EguiRunOptions,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    render_ctx: Option<RenderContext>,
    render_surface: Option<RenderSurface<'static>>,
    renderer: Option<Renderer>,
    scene: Scene,
}

impl NativeVelloRunner {
    fn new(options: EguiRunOptions) -> Self {
        Self {
            options,
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            scene: Scene::new(),
        }
    }

    fn build_window_attributes(&self) -> WindowAttributes {
        let mut attrs = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_maximized(self.options.maximized);
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

    fn initialize_runtime(&mut self, event_loop: &ActiveEventLoop) {
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                eprintln!("Failed to create native vello window: {err}");
                event_loop.exit();
                return;
            }
        };
        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let render_surface = match pollster::block_on(render_ctx.create_surface(
            window.clone(),
            width,
            height,
            wgpu::PresentMode::AutoVsync,
        )) {
            Ok(surface) => surface,
            Err(err) => {
                eprintln!("Failed to create native vello surface: {err}");
                event_loop.exit();
                return;
            }
        };
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        let renderer = match Renderer::new(&dev_handle.device, RendererOptions::default()) {
            Ok(renderer) => renderer,
            Err(err) => {
                eprintln!("Failed to create native vello renderer: {err}");
                event_loop.exit();
                return;
            }
        };
        self.window_id = Some(window.id());
        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.rebuild_scene();
    }

    fn rebuild_scene(&mut self) {
        self.scene.reset();
        let Some(surface) = self.render_surface.as_ref() else {
            return;
        };
        let width = surface.config.width as f64;
        let height = surface.config.height as f64;
        let margin = (width.min(height) * 0.08).max(8.0);
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(16, 18, 22),
            None,
            &Rect::new(0.0, 0.0, width, height),
        );
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(52, 74, 98),
            None,
            &Rect::new(margin, margin, width - margin, height - margin),
        );
        let radius = (width.min(height) * 0.18).max(12.0);
        self.scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(210, 156, 96),
            None,
            &Circle::new((width * 0.7, height * 0.35), radius),
        );
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.window.as_ref().cloned();
        let (Some(window), Some(render_ctx), Some(surface), Some(renderer)) = (
            window,
            self.render_ctx.as_mut(),
            self.render_surface.as_mut(),
            self.renderer.as_mut(),
        ) else {
            return;
        };
        let dev_handle = &render_ctx.devices[surface.dev_id];
        let surface_texture = match surface.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(err) => {
                match err {
                    wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                        let size = window.inner_size();
                        render_ctx.resize_surface(surface, size.width.max(1), size.height.max(1));
                        self.rebuild_scene();
                        window.request_redraw();
                    }
                    wgpu::SurfaceError::OutOfMemory => event_loop.exit(),
                    wgpu::SurfaceError::Timeout => {}
                    wgpu::SurfaceError::Other => {}
                }
                return;
            }
        };
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_result = renderer.render_to_texture(
            &dev_handle.device,
            &dev_handle.queue,
            &self.scene,
            &surface.target_view,
            &RenderParams {
                base_color: Color::BLACK,
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        );
        if let Err(err) = render_result {
            eprintln!("Native vello render failed: {err}");
            event_loop.exit();
            return;
        }
        let mut encoder = dev_handle
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("native_vello_present_blit"),
            });
        surface.blitter.copy(
            &dev_handle.device,
            &mut encoder,
            &surface.target_view,
            &surface_view,
        );
        dev_handle.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }
}

impl ApplicationHandler for NativeVelloRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                let window = self.window.as_ref().cloned();
                if size.width > 0
                    && size.height > 0
                    && let (Some(render_ctx), Some(surface), Some(window)) = (
                        self.render_ctx.as_ref(),
                        self.render_surface.as_mut(),
                        window,
                    )
                {
                    render_ctx.resize_surface(surface, size.width, size.height);
                    self.rebuild_scene();
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Wait);
    }
}

fn icon_from_rgba(icon: &WindowIconRgba) -> Option<Icon> {
    Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height).ok()
}

/// Run the experimental native Vello backend window for backend-selection testing.
///
/// This preview path renders a simple native scene and validates `winit + vello`
/// initialization without relying on `egui` UI composition.
pub fn run_native_vello_preview(options: EguiRunOptions) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    let mut runner = NativeVelloRunner::new(options);
    event_loop.run_app(&mut runner).map_err(|err| err.to_string())
}
