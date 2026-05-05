use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Reveal the startup window and request another redraw when the first present
    /// still needs to land on a backend that may have throttled hidden-window work.
    pub(in crate::gui_runtime::native_vello) fn reveal_startup_window(&mut self) {
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
            if !self.first_frame_presented {
                window.request_redraw();
                self.redraw_requested = true;
            }
        }
        self.startup_window_visible = true;
        self.startup_reveal_deadline = None;
        self.startup_timing.mark_window_revealed();
    }

    /// Reveal the native window after startup sequencing reaches a stable frame.
    pub(in crate::gui_runtime::native_vello) fn maybe_reveal_startup_window(&mut self) {
        if self.startup_window_visible || !self.first_frame_presented {
            return;
        }
        if self.startup_model_pull_pending || self.startup_deferred_model_refresh_pending {
            return;
        }
        self.reveal_startup_window();
    }

    /// Reveal the window once the first startup scene is ready.
    pub(in crate::gui_runtime::native_vello) fn maybe_reveal_startup_window_after_first_scene_ready(
        &mut self,
    ) {
        if self.startup_window_visible
            || self.first_frame_presented
            || self.startup_deferred_model_refresh_pending
        {
            return;
        }
        self.reveal_startup_window();
    }

    /// Force startup reveal when redraw delivery stalls while hidden.
    ///
    /// Some backends can throttle redraw delivery for hidden windows. This
    /// fallback ensures the app cannot remain hidden forever waiting on a
    /// second present.
    pub(in crate::gui_runtime::native_vello) fn maybe_force_reveal_startup_window_on_stall(
        &mut self,
        now: Instant,
    ) {
        if self.startup_window_visible {
            return;
        }
        let Some(deadline) = self.startup_reveal_deadline else {
            return;
        };
        if now < deadline {
            return;
        }
        warn!("native vello startup reveal fallback: forcing window visible after stall");
        self.reveal_startup_window();
    }

    /// Handle one successful first present and schedule deferred startup pulls.
    pub(in crate::gui_runtime::native_vello) fn complete_first_present(&mut self) {
        if !self.first_frame_presented {
            self.first_frame_presented = true;
            self.startup_timing.mark_first_presented();
            if !self.startup_window_visible {
                self.reveal_startup_window();
            }
            if self.startup_model_pull_pending {
                self.startup_model_pull_pending = false;
                self.startup_deferred_model_refresh_pending = true;
                self.apply_invalidation_scope(RuntimeInvalidationScope::ModelAndOverlays);
            }
        }
        self.maybe_reveal_startup_window();
        self.startup_timing.maybe_emit_summary();
    }

    pub(in crate::gui_runtime::native_vello) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if !self.first_frame_presented {
            self.startup_timing.mark_first_redraw_started();
        }
        self.redraw_count = self.redraw_count.saturating_add(1);
        self.redraw_requested = false;
        let now = Instant::now();
        let delta = (now - self.last_redraw).as_secs_f32();
        self.last_redraw = now;
        let frame_started_at = Instant::now();
        let frame_profile_start = self.profiler.now_if_enabled();
        let rebuild_start = self.profiler.now_if_enabled();
        let needs_animation = self.shell_state.needs_animation();
        let (has_rebuild, mut frame_result) = self.rebuild_scene_for_redraw(needs_animation, delta);
        let rebuild_duration = rebuild_start.map_or(Duration::ZERO, |start| start.elapsed());
        if self.redraw_count <= 8 {
            info!(
                "native vello redraw start: redraw_count={} needs_animation={} has_rebuild={} delta_ms={}",
                self.redraw_count,
                needs_animation,
                has_rebuild,
                ((delta * 1000.0) as u32)
            );
        }
        if !needs_animation && !has_rebuild && self.first_frame_presented {
            return;
        }

        let Some(window) = self.window.as_ref() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                false,
            );
            return;
        };
        let Some(dev_id) = self.render_surface.as_ref().map(|surface| surface.dev_id) else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                false,
            );
            return;
        };

        let mut surface_error = None;
        let mut needs_resize = false;
        let mut out_of_memory = false;
        let acquire_start = self.profiler.now_if_enabled();
        let surface_texture = {
            let Some(surface) = self.render_surface.as_mut() else {
                self.finish_redraw_attempt(
                    &mut frame_result,
                    frame_started_at,
                    frame_profile_start,
                    rebuild_duration,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    Duration::ZERO,
                    false,
                    false,
                );
                return;
            };
            match surface.surface.get_current_texture() {
                Ok(frame) => Some(frame),
                Err(err) => {
                    surface_error = Some(err.clone());
                    if self.redraw_count <= 8 {
                        warn!("native vello surface acquire error: {:?}", err);
                    }
                    match err {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            let size = window.inner_size();
                            if let Some(render_ctx) = self.render_ctx.as_mut() {
                                render_ctx.resize_surface(
                                    surface,
                                    size.width.max(1),
                                    size.height.max(1),
                                );
                                needs_resize = true;
                            }
                        }
                        wgpu::SurfaceError::OutOfMemory => out_of_memory = true,
                        wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {}
                    }
                    None
                }
            }
        };
        let acquire_duration = acquire_start.map_or(Duration::ZERO, |start| start.elapsed());
        if let Some(err) = surface_error {
            if out_of_memory {
                error!("native vello out-of-memory in surface acquire: {:?}", err);
            } else if self.redraw_count <= 8 {
                info!("native vello non-fatal surface error: {:?}", err);
            }
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            if matches!(err, wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated)
                && needs_resize
            {
                self.apply_invalidation_scope(RuntimeInvalidationScope::LayoutAndAll);
                self.rebuild_scene_if_needed();
            }
            if out_of_memory {
                event_loop.exit();
            }
            return;
        }
        let Some(surface_texture) = surface_texture else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };

        let Some(surface) = self.render_surface.as_mut() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let Some(render_ctx) = self.render_ctx.as_ref() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let Some(renderer) = self.renderer.as_mut() else {
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        };
        let dev_handle = &render_ctx.devices[dev_id];
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let render_start = self.profiler.now_if_enabled();
        let render_result = renderer.render_to_texture(
            &dev_handle.device,
            &dev_handle.queue,
            &self.scene,
            &surface.target_view,
            &RenderParams {
                base_color: color_from_rgba(self.clear_color),
                width: surface.config.width,
                height: surface.config.height,
                antialiasing_method: AaConfig::Area,
            },
        );
        if let Err(err) = render_result {
            error!("native vello render_to_texture failed: {:?}", err);
            event_loop.exit();
            let render = render_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.finish_redraw_attempt(
                &mut frame_result,
                frame_started_at,
                frame_profile_start,
                rebuild_duration,
                acquire_duration,
                render,
                Duration::ZERO,
                Duration::ZERO,
                false,
                true,
            );
            return;
        }
        let render_duration = render_start.map_or(Duration::ZERO, |start| start.elapsed());
        let blit_start = self.profiler.now_if_enabled();
        let mut encoder =
            dev_handle
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
        let blit_duration = blit_start.map_or(Duration::ZERO, |start| start.elapsed());
        let present_started_at = Instant::now();
        surface_texture.present();
        self.complete_first_present();
        let present_duration = present_started_at.elapsed();
        self.finish_redraw_attempt(
            &mut frame_result,
            frame_started_at,
            frame_profile_start,
            rebuild_duration,
            acquire_duration,
            render_duration,
            blit_duration,
            present_duration,
            true,
            true,
        );
    }
}
