//! Wavecrate GUI runtime host integration shared by the current `native_app` shell.

pub use radiant::gui_runtime::{
    DEFAULT_NATIVE_WINDOW_TITLE, NativeStartupTimingArtifact, RuntimeRunReport,
};

mod artifacts;
#[cfg(any(test, feature = "legacy-controller"))]
mod automation_snapshot;
mod config;

pub use artifacts::{NativeRunReport, NativeRuntimeArtifacts, NativeShutdownTimingArtifact};
#[cfg(any(test, feature = "legacy-controller"))]
pub use automation_snapshot::capture_gui_automation_snapshot;
pub use config::{
    NativeRunOptions, WAVECRATE_UI_FONT_ASSET, WAVECRATE_UI_FONT_BYTES, WindowIconRgba,
    wavecrate_ui_font_path,
};

#[cfg(test)]
mod tests {
    #[test]
    fn startup_timing_artifact_uses_generic_radiant_runtime_export() {
        let module = include_str!("mod.rs");
        let legacy_compat_export = concat!(
            "pub use radiant::compat::",
            "legacy_shell::NativeStartupTimingArtifact;"
        );

        assert!(module.contains("pub use radiant::gui_runtime::"));
        assert!(module.contains("NativeStartupTimingArtifact"));
        assert!(
            !module.contains(legacy_compat_export),
            "startup timing is a generic Radiant runtime artifact, not a Wavecrate compat DTO"
        );
    }
}
