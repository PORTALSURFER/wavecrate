//! Encoding and composition helpers for retained native-Vello scenes.

use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(crate) fn encode_frame_to_scene(
        frame: &NativeViewFrame,
        scene: &mut Scene,
        text_renderer: &mut NativeTextRenderer,
        image_upload_blob_cache: &mut HashMap<ImageUploadBlobCacheKey, Blob<u8>>,
        image_upload_blob_cache_order: &mut VecDeque<ImageUploadBlobCacheKey>,
    ) {
        scene.reset();
        for primitive in frame.primitives.iter() {
            match primitive {
                Primitive::Rect(fill) => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &to_kurbo_rect(fill.rect),
                    );
                }
                Primitive::Circle(fill) => {
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        color_from_rgba(fill.color),
                        None,
                        &Circle::new(
                            (fill.center.x as f64, fill.center.y as f64),
                            fill.radius as f64,
                        ),
                    );
                }
                Primitive::LinearGradient(fill) => {
                    let mut gradient = Gradient::new_linear(
                        KurboPoint::new(fill.start.x as f64, fill.start.y as f64),
                        KurboPoint::new(fill.end.x as f64, fill.end.y as f64),
                    );
                    gradient
                        .stops
                        .push((0.0, color_from_rgba(fill.start_color)).into());
                    gradient
                        .stops
                        .push((1.0, color_from_rgba(fill.end_color)).into());
                    scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        &gradient,
                        None,
                        &to_kurbo_rect(fill.rect),
                    );
                }
                Primitive::Image(draw) => {
                    let (Ok(width), Ok(height)) = (
                        u32::try_from(draw.image.width),
                        u32::try_from(draw.image.height),
                    ) else {
                        continue;
                    };
                    if width == 0
                        || height == 0
                        || draw.rect.width() <= 0.0
                        || draw.rect.height() <= 0.0
                    {
                        continue;
                    }
                    let blob = Self::cached_image_upload_blob(
                        image_upload_blob_cache,
                        image_upload_blob_cache_order,
                        &draw.image.pixels,
                        width,
                        height,
                    );
                    let image_data = ImageData {
                        data: blob,
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::Alpha,
                        width,
                        height,
                    };
                    let transform =
                        Affine::translate((draw.rect.min.x as f64, draw.rect.min.y as f64))
                            * Affine::scale_non_uniform(
                                draw.rect.width() as f64 / f64::from(width),
                                draw.rect.height() as f64 / f64::from(height),
                            );
                    scene.draw_image(&image_data, transform);
                }
            }
        }
        text_renderer.draw_text_runs(scene, &frame.text_runs);
    }

    pub(crate) fn rebuild_state_overlay_if_needed(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
        rebuild_requested: bool,
    ) -> bool {
        let hover_overlay_fingerprint = self.hover_overlay_cache_fingerprint(
            &self.model,
            style,
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
        );
        let focus_overlay_fingerprint = self.focus_overlay_cache_fingerprint(
            &self.model,
            style,
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
        );
        let modal_overlay_fingerprint = self.modal_overlay_cache_fingerprint(
            &self.model,
            style,
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
        );
        let rebuild_hover_overlay = rebuild_requested
            || self.hover_overlay_fingerprint.as_ref() != Some(&hover_overlay_fingerprint);
        let rebuild_focus_overlay = rebuild_requested
            || self.focus_overlay_fingerprint.as_ref() != Some(&focus_overlay_fingerprint);
        let rebuild_modal_overlay = rebuild_requested
            || self.modal_overlay_fingerprint.as_ref() != Some(&modal_overlay_fingerprint);
        if !rebuild_hover_overlay && !rebuild_focus_overlay && !rebuild_modal_overlay {
            return false;
        }

        let mut build_duration = Duration::ZERO;
        let mut encode_duration = Duration::ZERO;
        if rebuild_hover_overlay {
            self.hover_overlay_fingerprint = Some(hover_overlay_fingerprint);
            let build_start = self.profiler.now_if_enabled();
            self.shell_state.build_hover_overlay_into(
                layout,
                style,
                &self.model,
                &mut self.hover_overlay_frame_cache,
            );
            build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
            let encode_start = self.profiler.now_if_enabled();
            Self::encode_frame_to_scene(
                &self.hover_overlay_frame_cache,
                &mut self.hover_overlay_scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
        }
        if rebuild_focus_overlay {
            self.focus_overlay_fingerprint = Some(focus_overlay_fingerprint);
            let build_start = self.profiler.now_if_enabled();
            self.shell_state.build_focus_overlay_into(
                layout,
                style,
                &self.model,
                &mut self.focus_overlay_frame_cache,
            );
            build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
            let encode_start = self.profiler.now_if_enabled();
            Self::encode_frame_to_scene(
                &self.focus_overlay_frame_cache,
                &mut self.focus_overlay_scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
        }
        if rebuild_modal_overlay {
            self.modal_overlay_fingerprint = Some(modal_overlay_fingerprint);
            let build_start = self.profiler.now_if_enabled();
            self.shell_state.build_modal_overlay_into(
                layout,
                style,
                &self.model,
                &mut self.modal_overlay_frame_cache,
            );
            build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
            let encode_start = self.profiler.now_if_enabled();
            Self::encode_frame_to_scene(
                &self.modal_overlay_frame_cache,
                &mut self.modal_overlay_scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
        }
        self.profiler.add_build_state_overlay(build_duration);
        self.profiler.add_encode_state_overlay(encode_duration);
        true
    }

    pub(crate) fn resolve_motion_overlay_rebuild_flags(
        &mut self,
        style: &StyleTokens,
        layout_width_bits: u32,
        layout_height_bits: u32,
        layout_scale_bits: u32,
        rebuild_requested: bool,
    ) -> (bool, bool) {
        let mut rebuild_waveform_motion_overlay = rebuild_requested;
        let mut rebuild_chrome_motion_overlay = rebuild_requested;
        if let Some(motion_model) = self.motion_model.as_ref() {
            let waveform_motion_overlay_fingerprint = self
                .waveform_motion_overlay_cache_fingerprint(
                    motion_model,
                    style,
                    layout_width_bits,
                    layout_height_bits,
                    layout_scale_bits,
                );
            rebuild_waveform_motion_overlay |= self.waveform_motion_overlay_fingerprint.as_ref()
                != Some(&waveform_motion_overlay_fingerprint);
            if rebuild_waveform_motion_overlay {
                self.waveform_motion_overlay_fingerprint =
                    Some(waveform_motion_overlay_fingerprint);
            }
            let chrome_motion_overlay_fingerprint = self.chrome_motion_overlay_cache_fingerprint(
                motion_model,
                style,
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
            );
            rebuild_chrome_motion_overlay |= self.chrome_motion_overlay_fingerprint.as_ref()
                != Some(&chrome_motion_overlay_fingerprint);
            if rebuild_chrome_motion_overlay {
                self.chrome_motion_overlay_fingerprint = Some(chrome_motion_overlay_fingerprint);
            }
        }
        (
            rebuild_waveform_motion_overlay,
            rebuild_chrome_motion_overlay,
        )
    }

    pub(crate) fn rebuild_motion_overlays_if_needed(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        rebuild_requested: bool,
        rebuild_waveform_motion_overlay: bool,
        rebuild_chrome_motion_overlay: bool,
    ) {
        if rebuild_waveform_motion_overlay || rebuild_chrome_motion_overlay {
            let mut build_duration = Duration::ZERO;
            let mut encode_duration = Duration::ZERO;
            if rebuild_waveform_motion_overlay {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist before waveform-motion build");
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_waveform_motion_overlay_into(
                    layout,
                    style,
                    motion_model,
                    &mut self.waveform_motion_overlay_frame_cache,
                );
                build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.waveform_motion_overlay_frame_cache,
                    &mut self.waveform_motion_overlay_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            }
            if rebuild_chrome_motion_overlay {
                let motion_model = self
                    .motion_model
                    .as_ref()
                    .expect("motion model should exist before chrome-motion build");
                let build_start = self.profiler.now_if_enabled();
                self.shell_state.build_chrome_motion_overlay_into(
                    layout,
                    style,
                    motion_model,
                    &mut self.chrome_motion_overlay_frame_cache,
                );
                build_duration += build_start.map_or(Duration::ZERO, |start| start.elapsed());
                let encode_start = self.profiler.now_if_enabled();
                Self::encode_frame_to_scene(
                    &self.chrome_motion_overlay_frame_cache,
                    &mut self.chrome_motion_overlay_scene,
                    &mut self.text_renderer,
                    &mut self.image_upload_blob_cache,
                    &mut self.image_upload_blob_cache_order,
                );
                encode_duration += encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            }
            self.profiler.add_build_motion_overlay(build_duration);
            self.profiler.add_encode_motion_overlay(encode_duration);
        } else if rebuild_requested {
            self.profiler.add_motion_overlay_skip();
        }
    }

    pub(crate) fn compose_scene_layers(
        &mut self,
        rebuild_static: bool,
        rebuild_state_overlay: bool,
        rebuild_waveform_motion_overlay: bool,
        rebuild_chrome_motion_overlay: bool,
    ) {
        if rebuild_state_overlay {
            self.state_overlay_scene.reset();
            self.state_overlay_scene
                .append(&self.hover_overlay_scene, None);
            self.state_overlay_scene
                .append(&self.focus_overlay_scene, None);
        }
        if rebuild_waveform_motion_overlay || rebuild_chrome_motion_overlay {
            self.motion_overlay_scene.reset();
            self.motion_overlay_scene
                .append(&self.waveform_motion_overlay_scene, None);
            self.motion_overlay_scene
                .append(&self.chrome_motion_overlay_scene, None);
        }
        if rebuild_static
            || rebuild_state_overlay
            || rebuild_waveform_motion_overlay
            || rebuild_chrome_motion_overlay
        {
            self.scene.reset();
            self.scene.append(&self.static_scene, None);
            self.scene.append(&self.state_overlay_scene, None);
            self.scene.append(&self.motion_overlay_scene, None);
            // Modal/popover surfaces must draw last so pickers stay above
            // chrome-motion highlights, animated status adornments, and other
            // non-modal overlay passes.
            self.scene.append(&self.modal_overlay_scene, None);
        }
    }
}
