//! `winit + wgpu` host runtime for `egui` applications during migration off `eframe`.

use egui::{Context, ViewportCommand, ViewportId};
use egui_wgpu::{RendererOptions, WgpuConfiguration, wgpu, winit::Painter};
use egui_winit::State;
use std::{num::NonZeroU32, sync::Arc, time::Instant};
use vello::{AaConfig, RenderParams, Renderer, RendererOptions as VelloRendererOptions, Scene};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, Position, Size},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Icon, Window, WindowAttributes, WindowId},
};

/// RGBA icon bytes used to initialize a native window icon.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// Window configuration for [`run_egui_wgpu_app`].
#[derive(Clone, Debug)]
pub struct EguiRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
}

impl Default for EguiRunOptions {
    fn default() -> Self {
        Self {
            title: String::from("Sempal"),
            inner_size: None,
            min_inner_size: None,
            maximized: false,
            icon: None,
        }
    }
}

/// Runtime callbacks required by the shared `egui + wgpu` host.
pub trait EguiAppRuntime {
    /// Configure visuals or one-time state once `egui` context exists.
    fn setup(&mut self, _ctx: &Context) {}

    /// Update the app for one frame.
    fn update(&mut self, ctx: &Context, window: &Window);

    /// Called before runtime shutdown.
    fn on_exit(&mut self) {}

    /// Clear color for the root surface.
    fn clear_color(&self) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }
}

struct VelloScratch {
    renderer: Renderer,
    scene: Scene,
    _texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
}

impl VelloScratch {
    fn new(render_state: &egui_wgpu::RenderState) -> Option<Self> {
        let texture = render_state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("vello_runtime_scratch"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let renderer = Renderer::new(&render_state.device, VelloRendererOptions::default()).ok()?;
        Some(Self {
            renderer,
            scene: Scene::new(),
            _texture: texture,
            texture_view,
        })
    }

    fn render(&mut self, render_state: &egui_wgpu::RenderState) {
        let _ = self.renderer.render_to_texture(
            &render_state.device,
            &render_state.queue,
            &self.scene,
            &self.texture_view,
            &RenderParams {
                base_color: vello::peniko::Color::BLACK,
                width: 1,
                height: 1,
                antialiasing_method: AaConfig::Area,
            },
        );
    }
}

struct EguiWgpuRunner<A: EguiAppRuntime> {
    options: EguiRunOptions,
    app: A,
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    egui_ctx: Option<Context>,
    egui_state: Option<State>,
    painter: Option<Painter>,
    next_repaint_at: Option<Instant>,
    vello: Option<VelloScratch>,
}

impl<A: EguiAppRuntime> EguiWgpuRunner<A> {
    fn new(options: EguiRunOptions, app: A) -> Self {
        Self {
            options,
            app,
            window_id: None,
            window: None,
            egui_ctx: None,
            egui_state: None,
            painter: None,
            next_repaint_at: None,
            vello: None,
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
        if let Some(icon) = &self.options.icon
            && let Ok(icon) = Icon::from_rgba(icon.rgba.clone(), icon.width, icon.height)
        {
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
                eprintln!("Failed to create window: {err}");
                event_loop.exit();
                return;
            }
        };
        let ctx = Context::default();
        let mut state = State::new(
            ctx.clone(),
            ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );
        let mut painter = pollster::block_on(Painter::new(
            ctx.clone(),
            WgpuConfiguration::default(),
            false,
            RendererOptions::default(),
        ));
        if let Err(err) = pollster::block_on(painter.set_window(ViewportId::ROOT, Some(window.clone())))
        {
            eprintln!("Failed to initialize wgpu surface: {err}");
            event_loop.exit();
            return;
        }
        if let Some(max_side) = painter.max_texture_side() {
            state.set_max_texture_side(max_side);
        }
        self.app.setup(&ctx);
        self.window_id = Some(window.id());
        self.window = Some(window);
        self.egui_ctx = Some(ctx);
        self.egui_state = Some(state);
        self.painter = Some(painter);
        self.next_repaint_at = Some(Instant::now());
    }

    fn ensure_vello_scratch(&mut self) {
        if self.vello.is_some() {
            return;
        }
        let Some(painter) = self.painter.as_ref() else {
            return;
        };
        let Some(render_state) = painter.render_state() else {
            return;
        };
        self.vello = VelloScratch::new(&render_state);
    }

    fn apply_viewport_commands(
        event_loop: &ActiveEventLoop,
        window: &Window,
        commands: &[ViewportCommand],
    ) {
        for command in commands {
            match command {
                ViewportCommand::Close => event_loop.exit(),
                ViewportCommand::Title(title) => window.set_title(title),
                ViewportCommand::Focus => window.focus_window(),
                ViewportCommand::Maximized(maximized) => window.set_maximized(*maximized),
                ViewportCommand::MinInnerSize(size) => {
                    window.set_min_inner_size(Some(Size::Logical(LogicalSize::new(
                        size.x as f64,
                        size.y as f64,
                    ))));
                }
                ViewportCommand::InnerSize(size) => {
                    let _ = window.request_inner_size(Size::Logical(LogicalSize::new(
                        size.x as f64,
                        size.y as f64,
                    )));
                }
                ViewportCommand::OuterPosition(pos) => {
                    window.set_outer_position(PhysicalPosition::new(pos.x as i32, pos.y as i32));
                }
                ViewportCommand::Minimized(minimized) => window.set_minimized(*minimized),
                ViewportCommand::CursorVisible(visible) => window.set_cursor_visible(*visible),
                ViewportCommand::CursorPosition(pos) => {
                    let _ = window.set_cursor_position(Position::Logical(LogicalPosition::new(
                        pos.x as f64,
                        pos.y as f64,
                    )));
                }
                _ => {}
            }
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(egui_state), Some(egui_ctx)) = (
            self.window.as_ref(),
            self.egui_state.as_mut(),
            self.egui_ctx.as_ref(),
        ) else {
            return;
        };
        let raw_input = egui_state.take_egui_input(window);
        let full_output = egui_ctx.run(raw_input, |ctx| self.app.update(ctx, window));
        egui_state.handle_platform_output(window, full_output.platform_output.clone());

        let clipped_primitives = egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        {
            let Some(painter) = self.painter.as_mut() else {
                return;
            };
            painter.paint_and_update_textures(
                ViewportId::ROOT,
                full_output.pixels_per_point,
                self.app.clear_color(),
                &clipped_primitives,
                &full_output.textures_delta,
                Vec::new(),
            );
        }

        if let Some(root_viewport) = full_output.viewport_output.get(&ViewportId::ROOT) {
            Self::apply_viewport_commands(event_loop, window, &root_viewport.commands);
        }

        let render_state = self.painter.as_ref().and_then(Painter::render_state);
        self.ensure_vello_scratch();
        if let (Some(vello), Some(render_state)) = (self.vello.as_mut(), render_state) {
            vello.render(&render_state);
        }

        let repaint_delay = full_output
            .viewport_output
            .get(&ViewportId::ROOT)
            .map(|out| out.repaint_delay)
            .unwrap_or_default();
        self.next_repaint_at = Some(Instant::now() + repaint_delay);
    }
}

impl<A: EguiAppRuntime> ApplicationHandler for EguiWgpuRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.initialize_runtime(event_loop);
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
        let (Some(window), Some(egui_state), Some(painter)) = (
            self.window.as_ref(),
            self.egui_state.as_mut(),
            self.painter.as_mut(),
        ) else {
            return;
        };
        let response = egui_state.on_window_event(window, &event);
        if response.repaint {
            window.request_redraw();
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(width), Some(height)) =
                    (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
                {
                    painter.on_window_resized(ViewportId::ROOT, width, height);
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(next_repaint_at)) = (&self.window, self.next_repaint_at) {
            let now = Instant::now();
            if now >= next_repaint_at {
                window.request_redraw();
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_repaint_at));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(painter) = self.painter.as_mut() {
            painter.destroy();
        }
        self.app.on_exit();
    }
}

/// Run a single-window native `egui` app on top of `winit + wgpu`.
pub fn run_egui_wgpu_app<A: EguiAppRuntime + 'static>(
    options: EguiRunOptions,
    app: A,
) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|err| err.to_string())?;
    let mut runner = EguiWgpuRunner::new(options, app);
    event_loop.run_app(&mut runner).map_err(|err| err.to_string())
}
