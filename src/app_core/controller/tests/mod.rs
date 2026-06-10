use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::state::{SampleBrowserTab, UpdateStatus};
use crate::app_dirs::ConfigBaseGuard;
use crate::waveform::WaveformChannelView;

use super::{AppController, AppControllerUiRuntimeExt, WaveformRenderer};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use tempfile::{TempDir, tempdir};

/// Keeps one temporary config base alive so persisted controller test flows do
/// not write into the live library/config state.
struct ControllerPersistenceSandbox {
    _config_root: TempDir,
    _config_guard: ConfigBaseGuard,
}

impl ControllerPersistenceSandbox {
    /// Install one isolated config base for the current test thread.
    fn new() -> Self {
        let config_root = tempdir().expect("create isolated controller test config root");
        let config_guard = ConfigBaseGuard::set(config_root.path().to_path_buf());
        Self {
            _config_root: config_root,
            _config_guard: config_guard,
        }
    }
}

mod browser_sources;
mod contextual_actions;
mod dispatch;
mod persistence_boundary;
mod waveform;
