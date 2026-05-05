use super::*;
use crate::gui::repaint::{CoalescingRepaintSignal, RepaintSignal};

pub(crate) fn run_legacy_shell_vello_app_with_artifacts<B: NativeAppBridge>(
    options: NativeRunOptions,
    bridge: B,
) -> NativeRunReport {
    info!("radiant native vello: creating event loop");
    let run_started = Instant::now();
    let event_loop = match EventLoop::<RuntimeUserEvent>::with_user_event().build() {
        Ok(event_loop) => event_loop,
        Err(err) => {
            return NativeRunReport {
                artifacts: NativeRuntimeArtifacts::default(),
                result: Err(err.to_string()),
            };
        }
    };
    info!(
        "radiant native vello: event loop created with window_size={:?} min_window_size={:?} target_fps={}",
        options.inner_size, options.min_inner_size, options.target_fps
    );
    let mut runner = NativeVelloRunner::new(options, bridge);
    let proxy = event_loop.create_proxy();
    let repaint_signal: Arc<dyn RepaintSignal> = Arc::new(CoalescingRepaintSignal::new(
        Arc::clone(&runner.repaint_event_pending),
        move || proxy.send_event(RuntimeUserEvent::RepaintRequested).is_ok(),
    ));
    runner.bridge.install_repaint_signal(repaint_signal);
    info!("radiant native vello: runner initialized");
    let run_result = event_loop
        .run_app(&mut runner)
        .map_err(|err| err.to_string());
    let elapsed = run_started.elapsed();
    match &run_result {
        Ok(_) => info!(
            "radiant native vello: event loop ended in {} ms",
            elapsed.as_millis()
        ),
        Err(err) => warn!(
            "radiant native vello: event loop returned error in {} ms: {}",
            elapsed.as_millis(),
            err
        ),
    }
    info!("radiant native vello: event loop finished");
    let startup_timing = runner.startup_timing.export_artifact().map(|artifact| {
        radiant::gui_runtime::NativeStartupTimingArtifact {
            status: artifact.status,
            failure_reason: artifact.failure_reason,
            window_create_ms: artifact.window_create_ms,
            window_revealed_ms: artifact.window_revealed_ms,
            wgpu_surface_create_ms: artifact.wgpu_surface_create_ms,
            wgpu_device_ready_ms: artifact.wgpu_device_ready_ms,
            surface_ready_ms: artifact.surface_ready_ms,
            renderer_build_ms: artifact.renderer_build_ms,
            renderer_ready_ms: artifact.renderer_ready_ms,
            first_scene_ready_ms: artifact.first_scene_ready_ms,
            first_redraw_started_ms: artifact.first_redraw_started_ms,
            first_present_draw_ms: artifact.first_present_draw_ms,
            first_present_ms: artifact.first_present_ms,
            deferred_model_refresh_ms: artifact.deferred_model_refresh_ms,
            deferred_model_refresh_total_ms: artifact.deferred_model_refresh_total_ms,
        }
    });
    let shutdown_timing = runner
        .bridge
        .on_runtime_exit()
        .and_then(|value| serde_json::from_value(value).ok());
    let artifacts = NativeRuntimeArtifacts {
        startup_timing,
        shutdown_timing,
    };
    NativeRunReport {
        artifacts,
        result: run_result,
    }
}
