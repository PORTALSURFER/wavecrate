use super::super::*;
#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(in crate::gui_runtime::native_vello) fn new(options: NativeRunOptions, bridge: B) -> Self {
        let target_fps = options.target_fps.max(1);
        let frame_interval_ns = (1_000_000_000u64 / target_fps as u64).max(1);
        let target_frame_interval = Duration::from_nanos(frame_interval_ns);
        let focus_animation_interval =
            Duration::from_nanos((1_000_000_000u64 / FOCUS_PULSE_HZ).max(1));
        let idle_status_refresh_interval =
            Duration::from_nanos(1_000_000_000u64 / IDLE_STATUS_REFRESH_HZ.max(1));
        let cursor_activity_redraw_interval =
            Duration::from_nanos(1_000_000_000u64 / CURSOR_ACTIVITY_REDRAW_HZ.max(1));
        let startup_clear_color = Self::startup_placeholder_clear_color();
        let incremental_frame_pipeline =
            crate::env_flags::env_var_truthy(INCREMENTAL_FRAME_PIPELINE_ENV);
        info!(
            "radiant native vello runner created: title={} target_fps={} maximized={} has_icon={}",
            options.title,
            options.target_fps,
            options.maximized,
            options.icon.is_some()
        );
        Self {
            options,
            bridge,
            repaint_event_pending: Arc::new(AtomicBool::new(false)),
            incremental_frame_pipeline,
            model: Arc::new(AppModel::default()),
            window_id: None,
            window: None,
            render_ctx: None,
            render_surface: None,
            renderer: None,
            redraw_requested: false,
            frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            static_segment_frame_cache: StaticFrameSegments::default(),
            static_segment_graph: StaticSegmentStateGraph::default(),
            static_segment_scene_cache: StaticSegmentSceneCache::default(),
            hover_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            focus_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            modal_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            waveform_motion_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            chrome_motion_overlay_frame_cache: NativeViewFrame {
                clear_color: startup_clear_color,
                primitives: Vec::new(),
                text_runs: Vec::new(),
            },
            scene: Scene::new(),
            static_scene: Scene::new(),
            hover_overlay_scene: Scene::new(),
            focus_overlay_scene: Scene::new(),
            modal_overlay_scene: Scene::new(),
            state_overlay_scene: Scene::new(),
            waveform_motion_overlay_scene: Scene::new(),
            chrome_motion_overlay_scene: Scene::new(),
            motion_overlay_scene: Scene::new(),
            image_upload_blob_cache: HashMap::new(),
            image_upload_blob_cache_order: VecDeque::new(),
            hover_overlay_fingerprint: None,
            focus_overlay_fingerprint: None,
            modal_overlay_fingerprint: None,
            waveform_motion_overlay_fingerprint: None,
            chrome_motion_overlay_fingerprint: None,
            motion_model: None,
            motion_model_supported: true,
            segment_revisions: SegmentRevisions::default(),
            segment_revisions_supported: false,
            missing_segment_revision_fallback_applied: false,
            text_renderer: NativeTextRenderer::new(),
            style_cache: None,
            frame_state: NativeVelloFrameState {
                model_dirty: true,
                ..NativeVelloFrameState::default()
            },
            layout_runtime: ShellLayoutRuntime::default(),
            shell_layout: None,
            shell_state: NativeShellState::new(),
            clear_color: startup_clear_color,
            cursor_icon: CursorIcon::Default,
            last_cursor: None,
            pending_cursor: None,
            pending_hotkey_chord: None,
            pending_volume_milli: None,
            waveform_drag_mode: None,
            waveform_view_refresh_pending: false,
            waveform_click_seek_press: None,
            pending_browser_row_press: None,
            content_item_drag: None,
            selection_drag_active: false,
            last_emitted_waveform_drag_action: None,
            spatial_focus_drag_active: false,
            last_emitted_spatial_drag_content_id: None,
            folder_scrollbar_drag: None,
            browser_scrollbar_drag: None,
            last_emitted_browser_list_view_start: None,
            waveform_scrollbar_drag: None,
            waveform_pan_drag: None,
            last_emitted_waveform_view_center: None,
            volume_drag_active: false,
            last_emitted_volume_milli: None,
            modifiers: ModifiersState::default(),
            text_input_target: TextInputTarget::None,
            text_input_buffer: None,
            text_editor_state: None,
            browser_pill_editor_suggestion_index: None,
            active_text_field_visual_cache: None,
            text_input_drag_active: false,
            waveform_bpm_input_buffer: None,
            clipboard: None,
            clipboard_fallback_text: String::new(),
            last_redraw: Instant::now(),
            resumed_count: 0,
            window_event_count: 0,
            redraw_count: 0,
            first_frame_presented: false,
            startup_window_visible: false,
            startup_model_pull_pending: Self::startup_should_defer_first_model_pull(),
            startup_deferred_model_refresh_pending: false,
            startup_reveal_deadline: None,
            startup_timing: StartupTimingProfile::new(),
            target_frame_interval,
            focus_animation_interval,
            idle_status_refresh_interval,
            next_idle_status_refresh: Instant::now() + idle_status_refresh_interval,
            cursor_activity_redraw_interval,
            cursor_activity_redraw_until: None,
            model_refresh_count: 0,
            profiler: NativeVelloProfiler::new(),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn initialize_runtime(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) {
        info!("radiant native vello: initializing runtime window and surface");
        self.startup_timing.mark_init_started();
        let Some(window) = self.create_startup_window(event_loop) else {
            return;
        };
        self.window_id = Some(window.id());
        #[cfg(target_os = "windows")]
        self.install_external_drag_hwnd(window.as_ref());
        self.window = Some(Arc::clone(&window));
        self.maybe_reveal_startup_window_before_renderer_ready();

        let mut render_ctx = RenderContext::new();
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        info!(
            "radiant native vello: creating render surface with {}x{}",
            width, height
        );
        let Some(render_surface) =
            self.create_render_surface(event_loop, &mut render_ctx, &window, width, height)
        else {
            event_loop.exit();
            return;
        };
        self.startup_timing.mark_surface_ready();
        info!("radiant native vello: render surface created");
        let Some(renderer) = self.create_renderer(event_loop, &render_ctx, &render_surface) else {
            return;
        };

        self.window = Some(window);
        self.render_ctx = Some(render_ctx);
        self.render_surface = Some(render_surface);
        self.renderer = Some(renderer);
        self.frame_state.mark_layout_dirty();
        if self.startup_model_pull_pending {
            self.prepare_startup_first_frame_scene();
        } else {
            self.frame_state.mark_model_dirty();
        }
        self.rebuild_scene_if_needed();
        self.startup_timing.mark_first_scene_ready();
        self.maybe_reveal_startup_window_after_first_scene_ready();
        self.last_redraw = Instant::now();
    }

    fn create_startup_window(&mut self, event_loop: &ActiveEventLoop) -> Option<Arc<Window>> {
        let window = match event_loop.create_window(self.build_window_attributes()) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                error!("radiant native vello: failed to create window: {:?}", err);
                event_loop.exit();
                return None;
            }
        };
        self.startup_timing.mark_window_created();
        info!("radiant native vello: window created");
        self.arm_startup_reveal_deadline(Instant::now());
        Some(window)
    }

    fn create_render_surface(
        &mut self,
        event_loop: &ActiveEventLoop,
        render_ctx: &mut RenderContext,
        window: &Arc<Window>,
        width: u32,
        height: u32,
    ) -> Option<RenderSurface<'static>> {
        let surface = match render_ctx.instance.create_surface(window.clone()) {
            Ok(surface) => surface,
            Err(err) => {
                error!(
                    "radiant native vello: failed to create wgpu surface: {:?}",
                    err
                );
                event_loop.exit();
                return None;
            }
        };
        self.startup_timing.mark_wgpu_surface_created();
        let dev_id = match pollster::block_on(render_ctx.device(Some(&surface))) {
            Some(dev_id) => dev_id,
            None => {
                error!("radiant native vello: no compatible render device found");
                event_loop.exit();
                return None;
            }
        };
        self.startup_timing.mark_wgpu_device_ready();
        let supported_present_modes = surface
            .get_capabilities(render_ctx.devices[dev_id].adapter())
            .present_modes;
        let present_mode = select_present_mode(self.options.target_fps, &supported_present_modes);
        let preferred_present_mode = present_mode_candidates(self.options.target_fps)[0];
        match present_mode == preferred_present_mode {
            true => info!(
                "radiant native vello: selected {:?} present mode from supported {:?}",
                present_mode, supported_present_modes
            ),
            false => warn!(
                "radiant native vello: preferred {:?} present mode unavailable; using {:?} from supported {:?}",
                preferred_present_mode, present_mode, supported_present_modes
            ),
        }
        match pollster::block_on(render_ctx.create_render_surface(
            surface,
            width,
            height,
            present_mode,
        )) {
            Ok(render_surface) => Some(render_surface),
            Err(err) => {
                error!(
                    "radiant native vello: failed to create {:?} render surface: {:?}",
                    present_mode, err
                );
                event_loop.exit();
                None
            }
        }
    }

    fn create_renderer(
        &mut self,
        event_loop: &ActiveEventLoop,
        render_ctx: &RenderContext,
        render_surface: &RenderSurface<'_>,
    ) -> Option<Renderer> {
        let dev_handle = &render_ctx.devices[render_surface.dev_id];
        info!("radiant native vello: creating renderer");
        self.startup_timing.mark_renderer_started();
        let renderer = match Renderer::new(&dev_handle.device, startup_renderer_options()) {
            Ok(renderer) => renderer,
            Err(err) => {
                error!("radiant native vello: failed to create renderer: {:?}", err);
                event_loop.exit();
                return None;
            }
        };
        self.startup_timing.mark_renderer_ready();
        info!("radiant native vello: renderer created");
        Some(renderer)
    }

    #[cfg(target_os = "windows")]
    fn install_external_drag_hwnd(&mut self, window: &Window) {
        let Ok(handle) = window.window_handle() else {
            info!("radiant external drag: window handle unavailable during HWND install");
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            info!("radiant external drag: non-Win32 handle during HWND install");
            return;
        };
        info!(
            hwnd = handle.hwnd.get(),
            "radiant external drag: installing host HWND for external drag"
        );
        self.bridge.set_external_drag_hwnd(handle.hwnd.get());
    }
}
