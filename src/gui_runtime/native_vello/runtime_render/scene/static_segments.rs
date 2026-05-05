//! Static retained-scene rebuild planning and encoding helpers.

use super::super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    /// Rebuild and encode retained static segment scenes.
    pub(crate) fn rebuild_static_segment_scenes(
        &mut self,
        layout: &ShellLayout,
        style: &StyleTokens,
        dirty_segments: DirtySegments,
        segment_revisions: SegmentRevisions,
        force_rebuild: bool,
    ) -> (Duration, Duration) {
        if force_rebuild {
            self.static_segment_graph.clear();
        }
        let fingerprints = self.build_static_segment_fingerprints(layout, style, segment_revisions);
        let diff_plan = self
            .static_segment_graph
            .diff(dirty_segments, force_rebuild, fingerprints);
        let mut build_duration = Duration::ZERO;
        let mut encode_duration = Duration::ZERO;
        for segment in StaticFrameSegment::ALL {
            if !diff_plan.should_rebuild(segment) {
                continue;
            }

            let segment_build_start = Instant::now();
            self.shell_state.build_static_segment_with_style_into(
                layout,
                style,
                &self.model,
                self.motion_model.as_ref(),
                segment,
                &mut self.static_segment_frame_cache,
            );
            build_duration += segment_build_start.elapsed();

            let segment_encode_start = Instant::now();
            let frame = self.static_segment_frame_cache.frame(segment);
            let entry = self.static_segment_scene_cache.entry_mut(segment);
            Self::encode_frame_to_scene(
                frame,
                &mut entry.scene,
                &mut self.text_renderer,
                &mut self.image_upload_blob_cache,
                &mut self.image_upload_blob_cache_order,
            );
            encode_duration += segment_encode_start.elapsed();
            self.static_segment_graph
                .commit_segment(segment, diff_plan.fingerprint(segment));
        }

        self.frame_cache.clear_color = style.clear_color;
        self.static_segment_frame_cache
            .compose_into(&mut self.frame_cache);
        self.clear_color = self.frame_cache.clear_color;
        self.static_scene.reset();
        for segment in StaticFrameSegment::ALL {
            self.static_scene
                .append(self.static_segment_scene_cache.scene(segment), None);
        }
        (build_duration, encode_duration)
    }
}
