//! Scene rebuild orchestration for model refresh, dirty resolution, and composition.

use super::super::*;

#[derive(Clone, Copy)]
struct SceneRebuildRequests {
    model_refresh_requested: bool,
    static_rebuild_requested: bool,
    rebuild_static: bool,
    rebuild_state_overlay: bool,
    rebuild_motion_overlay: bool,
    layout_dirty_segments: DirtySegments,
    layout_rebuild: bool,
}

#[derive(Clone, Copy)]
struct SceneRefreshOutcome {
    bridge_dirty_segments: DirtySegments,
    rebuild_static: bool,
    rebuild_state_overlay: bool,
    rebuild_motion_overlay: bool,
}

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(in crate::gui_runtime::native_vello) fn rebuild_scene(
        &mut self,
        model_refresh_requested: bool,
        static_rebuild_requested: bool,
        rebuild_static: bool,
        rebuild_state_overlay: bool,
        rebuild_motion_overlay: bool,
        layout_dirty_segments: DirtySegments,
        layout_rebuild: bool,
    ) -> FrameBuildResult {
        let requests = SceneRebuildRequests {
            model_refresh_requested,
            static_rebuild_requested,
            rebuild_static,
            rebuild_state_overlay,
            rebuild_motion_overlay,
            layout_dirty_segments,
            layout_rebuild,
        };
        let refresh = self.refresh_scene_rebuild_outcome(requests);
        let Some(layout) = self.shell_layout.as_ref().map(Arc::clone) else {
            return self.frame_result_base();
        };
        let layout = layout.as_ref();
        let layout_width_bits = layout.root.rect.width().to_bits();
        let layout_height_bits = layout.root.rect.height().to_bits();
        let layout_scale_bits = layout.ui_scale.to_bits();
        let style = self.cached_style_for_layout(layout);

        if refresh.rebuild_static {
            self.rebuild_static_scene_if_needed(
                layout,
                &style,
                refresh.bridge_dirty_segments,
                requests.model_refresh_requested,
            );
        }

        let rebuild_state_overlay = self.rebuild_state_overlay_if_needed(
            layout,
            &style,
            layout_width_bits,
            layout_height_bits,
            layout_scale_bits,
            refresh.rebuild_state_overlay,
        );
        let (rebuild_waveform_motion_overlay, rebuild_chrome_motion_overlay) = self
            .resolve_motion_overlay_rebuild_flags(
                &style,
                layout_width_bits,
                layout_height_bits,
                layout_scale_bits,
                refresh.rebuild_motion_overlay,
            );
        self.rebuild_motion_overlays_if_needed(
            layout,
            &style,
            refresh.rebuild_motion_overlay,
            rebuild_waveform_motion_overlay,
            rebuild_chrome_motion_overlay,
        );
        self.compose_scene_layers(
            refresh.rebuild_static,
            rebuild_state_overlay,
            rebuild_waveform_motion_overlay,
            rebuild_chrome_motion_overlay,
        );
        self.frame_result_with_rebuilds(
            requests.layout_rebuild && refresh.rebuild_static,
            refresh.rebuild_static,
            rebuild_state_overlay,
            rebuild_waveform_motion_overlay || rebuild_chrome_motion_overlay,
        )
    }

    fn refresh_scene_rebuild_outcome(
        &mut self,
        requests: SceneRebuildRequests,
    ) -> SceneRefreshOutcome {
        let should_refresh_model = requests.model_refresh_requested
            || (!self.motion_model_supported && requests.rebuild_motion_overlay);
        let should_refresh_motion = requests.rebuild_motion_overlay && self.motion_model_supported;
        self.profiler.record_scene_rebuilds(
            requests.rebuild_static,
            requests.rebuild_state_overlay,
            requests.rebuild_motion_overlay,
        );
        if should_refresh_model {
            return self.refresh_full_model_for_scene(requests);
        }
        if should_refresh_motion {
            return self.refresh_motion_model_for_scene(requests);
        }
        SceneRefreshOutcome {
            bridge_dirty_segments: DirtySegments::all(),
            rebuild_static: requests.rebuild_static,
            rebuild_state_overlay: requests.rebuild_state_overlay,
            rebuild_motion_overlay: requests.rebuild_motion_overlay,
        }
    }

    fn refresh_full_model_for_scene(
        &mut self,
        requests: SceneRebuildRequests,
    ) -> SceneRefreshOutcome {
        self.profiler.add_bridge_model_pull_rebuild();
        let pull_start = self.profiler.now_if_enabled();
        self.profiler.add_model_refresh();
        self.model_refresh_count = self.model_refresh_count.saturating_add(1);
        if self.model_refresh_count <= 24 {
            info!(
                "native vello refreshing model: refresh_count={} rebuild_static={} rebuild_state_overlay={} rebuild_motion_overlay={}",
                self.model_refresh_count,
                requests.rebuild_static,
                requests.rebuild_state_overlay,
                requests.rebuild_motion_overlay
            );
        }
        self.model = self.bridge.project_model();
        self.waveform_view_refresh_pending = false;
        let mut bridge_dirty_segments = self.bridge.take_dirty_segments();
        bridge_dirty_segments.insert(requests.layout_dirty_segments.bits());
        self.refresh_segment_revisions_from_bridge();
        let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
        self.profiler.add_model_pull(pull_duration);
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.motion_model_supported = true;
        self.sync_text_input_target();
        self.finish_deferred_startup_refresh_if_needed();
        let rebuild_static = resolve_static_rebuild(
            requests.model_refresh_requested,
            requests.static_rebuild_requested,
            bridge_dirty_segments,
        );
        if static_rebuild_from_dirty_mask(
            requests.model_refresh_requested,
            requests.static_rebuild_requested,
            bridge_dirty_segments,
        ) {
            self.profiler.add_dirty_mask_static_rebuild();
        }
        SceneRefreshOutcome {
            bridge_dirty_segments,
            rebuild_static,
            rebuild_state_overlay: requests.rebuild_state_overlay,
            rebuild_motion_overlay: requests.rebuild_motion_overlay,
        }
    }

    fn refresh_motion_model_for_scene(
        &mut self,
        requests: SceneRebuildRequests,
    ) -> SceneRefreshOutcome {
        let previous_waveform_signature = self
            .motion_model
            .as_ref()
            .and_then(|model| model.waveform_image_signature);
        self.profiler.add_bridge_motion_pull_rebuild();
        let pull_start = self.profiler.now_if_enabled();
        if let Some(motion_model) = self.bridge.project_motion_model() {
            let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_motion_pull(pull_duration);
            let mut rebuild_static = requests.rebuild_static;
            let mut rebuild_state_overlay = requests.rebuild_state_overlay;
            let mut rebuild_motion_overlay = requests.rebuild_motion_overlay;
            if self.motion_model.as_ref() != Some(&motion_model) {
                if previous_waveform_signature != motion_model.waveform_image_signature {
                    rebuild_static = true;
                    rebuild_state_overlay = true;
                    rebuild_motion_overlay = true;
                }
                self.shell_state.sync_from_motion_model(&motion_model);
                self.motion_model = Some(motion_model);
            }
            return SceneRefreshOutcome {
                bridge_dirty_segments: DirtySegments::all(),
                rebuild_static,
                rebuild_state_overlay,
                rebuild_motion_overlay,
            };
        }

        let pull_duration = pull_start.map_or(Duration::ZERO, |start| start.elapsed());
        self.profiler.add_motion_pull(pull_duration);
        let model_pull_start = self.profiler.now_if_enabled();
        self.profiler.add_bridge_model_pull_rebuild();
        self.motion_model_supported = false;
        self.model = self.bridge.project_model();
        self.waveform_view_refresh_pending = false;
        let mut bridge_dirty_segments = self.bridge.take_dirty_segments();
        bridge_dirty_segments.insert(requests.layout_dirty_segments.bits());
        self.refresh_segment_revisions_from_bridge();
        let model_pull_duration = model_pull_start.map_or(Duration::ZERO, |start| start.elapsed());
        self.profiler.add_model_pull(model_pull_duration);
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.sync_text_input_target();
        self.finish_deferred_startup_refresh_if_needed();
        SceneRefreshOutcome {
            bridge_dirty_segments,
            rebuild_static: requests.rebuild_static,
            rebuild_state_overlay: requests.rebuild_state_overlay,
            rebuild_motion_overlay: requests.rebuild_motion_overlay,
        }
    }

    fn refresh_segment_revisions_from_bridge(&mut self) {
        let bridge_segment_revisions = self.bridge.take_segment_revisions();
        if bridge_segment_revisions.has_static_revisions() {
            self.segment_revisions_supported = true;
        }
        if self.segment_revisions_supported {
            self.segment_revisions = bridge_segment_revisions;
        }
    }

    fn finish_deferred_startup_refresh_if_needed(&mut self) {
        if self.startup_deferred_model_refresh_pending {
            self.startup_deferred_model_refresh_pending = false;
            self.startup_reveal_deadline = None;
            self.startup_timing.mark_deferred_model_refresh_done();
            self.startup_timing.maybe_emit_summary();
        }
    }

    fn rebuild_static_scene_if_needed(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        bridge_dirty_segments: DirtySegments,
        model_refresh_requested: bool,
    ) {
        if self.incremental_frame_pipeline {
            let mut force_rebuild = !model_refresh_requested;
            if !self.segment_revisions_supported && !self.missing_segment_revision_fallback_applied
            {
                warn!(
                    "native vello bridge reported zero segment revisions; forcing one conservative static rebuild"
                );
                force_rebuild = true;
                self.missing_segment_revision_fallback_applied = true;
            }
            let (build_duration, encode_duration) = self.rebuild_static_segment_scenes(
                layout,
                style,
                bridge_dirty_segments,
                self.segment_revisions,
                force_rebuild,
            );
            self.profiler.add_build_static(build_duration);
            self.profiler.add_encode_static(encode_duration);
        } else {
            let build_start = self.profiler.now_if_enabled();
            self.frame_cache.clear_color = style.clear_color;
            self.shell_state.build_frame_with_style_into_static(
                layout,
                style,
                &self.model,
                &mut self.frame_cache,
            );
            let build_duration = build_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_build_static(build_duration);
            let encode_start = self.profiler.now_if_enabled();
            Self::encode_frame_to_scene(
                &self.frame_cache,
                &mut self.static_scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            let encode_duration = encode_start.map_or(Duration::ZERO, |start| start.elapsed());
            self.profiler.add_encode_static(encode_duration);
            self.clear_color = self.frame_cache.clear_color;
        }
        self.sync_browser_viewport_from_shell(layout);
    }
}
