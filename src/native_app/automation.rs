//! Product-composition GUI automation harness for startup certification.

use std::{sync::Arc, time::Duration};

use radiant::{
    gui::{automation::GuiAutomationSnapshot, repaint::RepaintSignal},
    layout::Vector2,
    runtime::SurfaceRuntime,
};
use serde::Serialize;

use crate::native_app::{
    app::NativeAppState, app_chrome::view_models::sample_browser::prepare_sample_browser_view,
    shell::native_app_runtime_bridge,
};

/// Observable worker composition started by one native app state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct NativeRuntimeComposition {
    /// Number of native filesystem watcher coordinators owned by the state.
    pub(crate) source_watcher_count: usize,
    /// Number of native readiness supervisors owned by the state.
    pub(crate) readiness_supervisor_count: usize,
    /// Number of retained controller analysis pools started by the state.
    pub(crate) legacy_analysis_pool_count: usize,
}

/// Product-native startup artifact captured through Radiant's runtime boundary.
pub(crate) struct NativeAutomationCapture {
    /// Backend-neutral semantic snapshot projected by the real native view.
    pub(crate) automation_snapshot: GuiAutomationSnapshot,
    /// Background runtime composition observed before shutdown.
    pub(crate) runtime_composition: NativeRuntimeComposition,
    /// Artifact returned by the real native shutdown hook.
    pub(crate) shutdown_artifact: Option<serde_json::Value>,
}

/// Capture one product-native startup without opening a desktop window.
pub(crate) fn capture_startup(viewport: [u32; 2]) -> Result<NativeAutomationCapture, String> {
    let mut state = NativeAppState::load_for_automation()?;
    state.wait_for_source_watcher_ready(Duration::from_secs(30))?;
    let runtime_composition = state.automation_runtime_composition();
    prepare_sample_browser_view(&mut state);

    let bridge = native_app_runtime_bridge(state);
    let mut runtime =
        SurfaceRuntime::new(bridge, Vector2::new(viewport[0] as f32, viewport[1] as f32));
    runtime.host_install_repaint_signal(Arc::new(NoopRepaintSignal));
    let _ = runtime.drain_runtime_messages();
    let automation_snapshot = runtime.automation_snapshot();
    let shutdown_artifact = runtime.host_on_runtime_exit();

    Ok(NativeAutomationCapture {
        automation_snapshot,
        runtime_composition,
        shutdown_artifact,
    })
}

struct NoopRepaintSignal;

impl RepaintSignal for NoopRepaintSignal {
    fn request_repaint(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_dirs::{ConfigBaseGuard, PersistenceProfileGuard};
    use crate::sample_sources::{
        SampleSource,
        config::{AppConfig, AppSettingsCore, save},
    };
    use tempfile::tempdir;

    #[test]
    fn startup_capture_uses_native_workers_and_shutdown_hook() {
        let config_base = tempdir().expect("config base");
        let source_root = tempdir().expect("source root");
        let _base_guard = ConfigBaseGuard::set(config_base.path().to_path_buf());
        let _profile_guard = PersistenceProfileGuard::automated();
        save(&AppConfig {
            sources: vec![SampleSource::new(source_root.path().to_path_buf())],
            core: AppSettingsCore::default(),
        })
        .expect("seed isolated startup profile");

        let capture = capture_startup([960, 540]).expect("native startup capture");

        assert_eq!(
            capture.runtime_composition,
            NativeRuntimeComposition {
                source_watcher_count: 1,
                readiness_supervisor_count: 1,
                legacy_analysis_pool_count: 0,
            }
        );
        assert_eq!(capture.automation_snapshot.viewport_width, 960);
        assert_eq!(capture.automation_snapshot.viewport_height, 540);
        assert_eq!(
            capture
                .shutdown_artifact
                .as_ref()
                .and_then(|artifact| artifact["source_processing"]["joined"].as_bool()),
            Some(true)
        );
    }
}
