use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::state::{SampleBrowserTab, UpdateStatus};
use crate::waveform::WaveformChannelView;

use super::{AppController, AppControllerNativeRuntimeExt, WaveformRenderer};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;

mod browser_sources;
mod dispatch;
mod waveform;
