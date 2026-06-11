//! Test-only native-app accessors.

mod audio;
mod config;
mod context_menu;
mod sample_browser;
mod settings;
mod shell;
mod state;
mod toolbar;
mod waveform;

pub(in crate::native_app) use audio::*;
pub(in crate::native_app) use config::*;
pub(in crate::native_app) use context_menu::*;
pub(in crate::native_app) use sample_browser::*;
pub(in crate::native_app) use settings::*;
pub(in crate::native_app) use shell::*;
pub(in crate::native_app) use state::*;
pub(in crate::native_app) use toolbar::*;
pub(in crate::native_app) use waveform::*;
