mod actions;
mod delegates;
pub(crate) mod helpers;

pub(crate) use actions::BrowserActions;
pub(crate) use helpers::BrowserController;

use super::*;
use crate::app::controller::library::wav_io::file_metadata;
use std::path::{Path, PathBuf};
