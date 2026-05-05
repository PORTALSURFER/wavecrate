use super::super::test_support::{dummy_controller, sample_entry, write_test_wav};
use super::super::*;
use super::common::{prepare_browser_sample, visible_indices};
use crate::app::controller::ui::hotkeys;
use crate::app::state::{FocusContext, SampleBrowserTab};
use crate::gui::input::KeyCode;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use std::path::Path;
use tempfile::tempdir;

mod browser_focus;
mod history;
mod mutation_hotkeys;
mod random_navigation;
