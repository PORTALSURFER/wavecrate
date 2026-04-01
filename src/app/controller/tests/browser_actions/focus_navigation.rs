use super::super::super::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::controller::ui::hotkeys;
use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod commit_focus;
mod preview_focus;
mod viewport_navigation;
