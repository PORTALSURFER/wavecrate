//! Runtime and CLI configuration for deterministic GUI test flows.

use crate::{app_dirs, env_flags::env_var_truthy, gui_runtime::NativeRunOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const GUI_TEST_MODE_ENV: &str = "SEMPAL_GUI_TEST_MODE";
const GUI_TEST_ARTIFACT_DIR_ENV: &str = "SEMPAL_GUI_TEST_ARTIFACT_DIR";
const GUI_TEST_FIXTURE_ENV: &str = "SEMPAL_GUI_TEST_FIXTURE";
const GUI_TEST_VIEWPORT_ENV: &str = "SEMPAL_GUI_TEST_VIEWPORT";
const GUI_TEST_SCENARIO_ENV: &str = "SEMPAL_GUI_TEST_SCENARIO";

/// Deterministic runtime settings used by GUI contract tools and app test mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiTestModeConfig {
    /// Fixed logical viewport used for snapshot and runtime test captures.
    pub viewport: [u32; 2],
    /// Directory where machine-readable GUI artifacts are written.
    pub artifact_dir: PathBuf,
    /// Optional scenario name associated with the current run.
    pub scenario_name: Option<String>,
    /// Fixture tag used to label the current test seed state.
    pub fixture_tag: String,
    /// Deterministic seed reserved for future fixture randomization.
    pub deterministic_seed: u64,
    /// Whether nonessential animations should be treated as disabled for test runs.
    pub disable_nonessential_animations: bool,
    /// Background-job policy label for the run.
    pub background_job_policy: String,
    /// Run-contract id correlated with this GUI test run, when available.
    pub run_id: Option<String>,
    /// Run-contract manifest path correlated with this GUI test run, when available.
    pub run_manifest_path: Option<PathBuf>,
}

impl Default for GuiTestModeConfig {
    fn default() -> Self {
        Self {
            viewport: [1440, 810],
            artifact_dir: PathBuf::from("gui-test"),
            scenario_name: None,
            fixture_tag: String::from("default"),
            deterministic_seed: 1,
            disable_nonessential_animations: true,
            background_job_policy: String::from("foreground_only"),
            run_id: None,
            run_manifest_path: None,
        }
    }
}

impl GuiTestModeConfig {
    /// Resolve GUI test mode from environment variables.
    pub fn from_env(run_id: Option<&str>, run_manifest_path: Option<PathBuf>) -> Option<Self> {
        if !env_var_truthy(GUI_TEST_MODE_ENV) {
            return None;
        }
        let mut config = Self::default();
        config.run_id = run_id.map(String::from);
        config.run_manifest_path = run_manifest_path;
        config.scenario_name = std::env::var(GUI_TEST_SCENARIO_ENV).ok();
        config.fixture_tag = std::env::var(GUI_TEST_FIXTURE_ENV)
            .unwrap_or_else(|_| String::from("default"));
        if let Ok(value) = std::env::var(GUI_TEST_VIEWPORT_ENV)
            && let Some(viewport) = parse_viewport(&value)
        {
            config.viewport = viewport;
        }
        config.artifact_dir = std::env::var(GUI_TEST_ARTIFACT_DIR_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                app_dirs::logs_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("gui-test")
            });
        Some(config)
    }

    /// Apply deterministic window sizing to native runtime launch options.
    pub fn apply_to_run_options(&self, options: &mut NativeRunOptions) {
        let viewport = [self.viewport[0] as f32, self.viewport[1] as f32];
        options.inner_size = Some(viewport);
        options.min_inner_size = Some(viewport);
        options.maximized = false;
        options.decorations = false;
    }

    /// Return the configured viewport in `f32` logical coordinates.
    pub fn viewport_f32(&self) -> [f32; 2] {
        [self.viewport[0] as f32, self.viewport[1] as f32]
    }
}

fn parse_viewport(value: &str) -> Option<[u32; 2]> {
    let mut parts = value.split('x');
    let width = parts.next()?.trim().parse::<u32>().ok()?;
    let height = parts.next()?.trim().parse::<u32>().ok()?;
    (width > 0 && height > 0).then_some([width, height])
}
