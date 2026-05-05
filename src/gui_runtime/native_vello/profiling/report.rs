//! Feature-gated redraw reporting and reset helpers for native-Vello profiling.

use super::stats::{InteractionProfileStats, NativeVelloProfiler};
use super::*;

impl NativeVelloProfiler {
    pub(in crate::gui_runtime::native_vello) fn record_redraw(
        &mut self,
        rebuild: Duration,
        acquire: Duration,
        render: Duration,
        blit: Duration,
        present: Duration,
        total: Duration,
        text_profile: (u64, u64, u64, u64, u64, u64),
    ) {
        if !self.enabled {
            return;
        }
        self.frames = self.frames.saturating_add(1);
        self.rebuild_ns = self.rebuild_ns.saturating_add(rebuild.as_nanos());
        self.acquire_ns = self.acquire_ns.saturating_add(acquire.as_nanos());
        self.render_ns = self.render_ns.saturating_add(render.as_nanos());
        self.blit_ns = self.blit_ns.saturating_add(blit.as_nanos());
        self.present_ns = self.present_ns.saturating_add(present.as_nanos());
        self.total_ns = self.total_ns.saturating_add(total.as_nanos());

        if self.frames < REDRAW_PROFILE_INTERVAL_FRAMES {
            return;
        }

        let frames = self.frames as f64;
        let total_ns = self.total_ns as f64;
        if total_ns <= 0.0 {
            self.reset();
            return;
        }

        let ms = |value_ns: u128| value_ns as f64 / 1_000_000.0;
        let avg_total_ms = ms(self.total_ns) / frames;
        let avg_rebuild_ms = ms(self.rebuild_ns) / frames;
        let avg_acquire_ms = ms(self.acquire_ns) / frames;
        let avg_render_ms = ms(self.render_ns) / frames;
        let avg_blit_ms = ms(self.blit_ns) / frames;
        let avg_present_ms = ms(self.present_ns) / frames;
        let avg_model_pull_ms = ms(self.model_pull_ns) / frames;
        let avg_motion_pull_ms = ms(self.motion_pull_ns) / frames;
        let avg_tick_ms = ms(self.tick_ns) / frames;
        let avg_build_static_ms = ms(self.build_static_ns) / frames;
        let avg_build_state_overlay_ms = ms(self.build_state_overlay_ns) / frames;
        let avg_build_motion_overlay_ms = ms(self.build_motion_overlay_ns) / frames;
        let avg_encode_static_ms = ms(self.encode_static_ns) / frames;
        let avg_encode_state_overlay_ms = ms(self.encode_state_overlay_ns) / frames;
        let avg_encode_motion_overlay_ms = ms(self.encode_motion_overlay_ns) / frames;
        let fps = 1000.0 / avg_total_ms.max(0.001);
        let rebuild_pct = (self.rebuild_ns as f64) * 100.0 / total_ns;
        let acquire_pct = (self.acquire_ns as f64) * 100.0 / total_ns;
        let render_pct = (self.render_ns as f64) * 100.0 / total_ns;
        let blit_pct = (self.blit_ns as f64) * 100.0 / total_ns;
        let present_pct = (self.present_ns as f64) * 100.0 / total_ns;
        let model_refresh_avg = self.model_refreshes as f64 / frames;
        let scene_rebuild_avg = self.scene_rebuilds as f64 / frames;
        let state_overlay_rebuild_avg = self.state_overlay_rebuilds as f64 / frames;
        let motion_overlay_rebuild_avg = self.motion_overlay_rebuilds as f64 / frames;
        let motion_overlay_skip_avg = self.motion_overlay_skips as f64 / frames;
        let bridge_model_pull_rebuild_avg = self.bridge_model_pull_rebuilds as f64 / frames;
        let bridge_motion_pull_rebuild_avg = self.bridge_motion_pull_rebuilds as f64 / frames;
        let explicit_static_rebuild_avg = self.explicit_static_rebuilds as f64 / frames;
        let dirty_mask_static_rebuild_avg = self.dirty_mask_static_rebuilds as f64 / frames;
        let (text_hits, text_misses, text_evictions, atom_hits, atom_misses, atom_evictions) =
            text_profile;
        let text_cache_hit_rate = cache_rate(text_hits, text_misses);
        let text_cache_miss_rate = cache_rate(text_misses, text_hits);
        let atom_cache_hit_rate = cache_rate(atom_hits, atom_misses);
        let atom_cache_miss_rate = cache_rate(atom_misses, atom_hits);
        eprintln!(
            "[native-vello] redraw avg over {REDRAW_PROFILE_INTERVAL_FRAMES} frames: \
             total={avg_total_ms:.2}ms ({fps:.1} fps) rebuild={avg_rebuild_ms:.2}ms ({rebuild_pct:.1}%) \
             acquire={avg_acquire_ms:.2}ms ({acquire_pct:.1}%) render={avg_render_ms:.2}ms ({render_pct:.1}%) \
             blit={avg_blit_ms:.2}ms ({blit_pct:.1}%) present={avg_present_ms:.2}ms ({present_pct:.1}%) \
             model_refresh_avg={model_refresh_avg:.2} scene_rebuild_avg={scene_rebuild_avg:.2} \
             state_overlay_rebuild_avg={state_overlay_rebuild_avg:.2} motion_overlay_rebuild_avg={motion_overlay_rebuild_avg:.2} motion_overlay_skip_avg={motion_overlay_skip_avg:.2} \
             bridge_model_pull_rebuild_avg={bridge_model_pull_rebuild_avg:.2} bridge_motion_pull_rebuild_avg={bridge_motion_pull_rebuild_avg:.2} \
             explicit_static_rebuild_avg={explicit_static_rebuild_avg:.2} dirty_mask_static_rebuild_avg={dirty_mask_static_rebuild_avg:.2} \
             model_pull_ms={avg_model_pull_ms:.3} motion_pull_ms={avg_motion_pull_ms:.3} tick_ms={avg_tick_ms:.3} \
             build_static_ms={avg_build_static_ms:.3} build_state_overlay_ms={avg_build_state_overlay_ms:.3} build_motion_overlay_ms={avg_build_motion_overlay_ms:.3} \
             encode_static_ms={avg_encode_static_ms:.3} encode_state_overlay_ms={avg_encode_state_overlay_ms:.3} encode_motion_overlay_ms={avg_encode_motion_overlay_ms:.3} \
             hover_samples={} hover_avg_ms={:.3} hover_max_ms={:.3} wheel_samples={} wheel_avg_ms={:.3} wheel_max_ms={:.3} \
             spatial_proxy_samples={} spatial_proxy_avg_ms={:.3} spatial_proxy_max_ms={:.3} timeline_samples={} timeline_avg_ms={:.3} timeline_max_ms={:.3} \
             volume_samples={} volume_avg_ms={:.3} volume_max_ms={:.3} \
             text_layout_hits={text_hits} text_layout_misses={text_misses} text_layout_evictions={text_evictions} text_hit_rate={text_cache_hit_rate:.1}% text_miss_rate={text_cache_miss_rate:.1}% \
             text_atom_hits={atom_hits} text_atom_misses={atom_misses} text_atom_evictions={atom_evictions} text_atom_hit_rate={atom_cache_hit_rate:.1}% text_atom_miss_rate={atom_cache_miss_rate:.1}%",
            self.hover_latency.samples,
            self.hover_latency.avg_ms(),
            self.hover_latency.max_ms(),
            self.wheel_latency.samples,
            self.wheel_latency.avg_ms(),
            self.wheel_latency.max_ms(),
            self.spatial_pan_proxy_latency.samples,
            self.spatial_pan_proxy_latency.avg_ms(),
            self.spatial_pan_proxy_latency.max_ms(),
            self.timeline_latency.samples,
            self.timeline_latency.avg_ms(),
            self.timeline_latency.max_ms(),
            self.volume_latency.samples,
            self.volume_latency.avg_ms(),
            self.volume_latency.max_ms(),
        );
        self.reset();
    }

    fn reset(&mut self) {
        self.frames = 0;
        self.rebuild_ns = 0;
        self.acquire_ns = 0;
        self.render_ns = 0;
        self.blit_ns = 0;
        self.present_ns = 0;
        self.total_ns = 0;
        self.scene_rebuilds = 0;
        self.state_overlay_rebuilds = 0;
        self.motion_overlay_rebuilds = 0;
        self.model_refreshes = 0;
        self.model_pull_ns = 0;
        self.motion_pull_ns = 0;
        self.bridge_model_pull_rebuilds = 0;
        self.bridge_motion_pull_rebuilds = 0;
        self.explicit_static_rebuilds = 0;
        self.dirty_mask_static_rebuilds = 0;
        self.tick_ns = 0;
        self.build_static_ns = 0;
        self.build_state_overlay_ns = 0;
        self.build_motion_overlay_ns = 0;
        self.encode_static_ns = 0;
        self.encode_state_overlay_ns = 0;
        self.encode_motion_overlay_ns = 0;
        self.motion_overlay_skips = 0;
        self.hover_latency = InteractionProfileStats::default();
        self.wheel_latency = InteractionProfileStats::default();
        self.spatial_pan_proxy_latency = InteractionProfileStats::default();
        self.timeline_latency = InteractionProfileStats::default();
        self.volume_latency = InteractionProfileStats::default();
    }
}

fn cache_rate(hits: u64, misses: u64) -> f64 {
    if hits + misses == 0 {
        0.0
    } else {
        100.0 * (hits as f64) / (hits + misses) as f64
    }
}
