//! Source/folder sidebar projection helpers.

use super::*;
use crate::app_core::app_api::state::{FolderBrowserUiState, FolderPaneId};
use std::path::{Path, PathBuf};

mod folder_name_validation;
mod folder_panes;
mod inline_edit;
mod panel;
mod source_rows;
mod tree_rows;

pub(crate) use panel::project_sources_model;
