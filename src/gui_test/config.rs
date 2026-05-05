//! Runtime and CLI configuration for deterministic GUI test flows.

use crate::{app_dirs, env_flags::env_var_truthy, gui_runtime::NativeRunOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const GUI_TEST_MODE_ENV: &str = "SEMPAL_GUI_TEST_MODE";
const GUI_TEST_ARTIFACT_DIR_ENV: &str = "SEMPAL_GUI_TEST_ARTIFACT_DIR";
const GUI_TEST_FIXTURE_ENV: &str = "SEMPAL_GUI_TEST_FIXTURE";
const GUI_TEST_VIEWPORT_ENV: &str = "SEMPAL_GUI_TEST_VIEWPORT";
const GUI_TEST_SCENARIO_ENV: &str = "SEMPAL_GUI_TEST_SCENARIO";
const LEGACY_DEFAULT_GUI_TEST_FIXTURE_TAG: &str = "default";

/// Canonical GUI fixture tag that exercises persisted startup in an isolated profile.
pub const GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG: &str = "isolated-startup";
/// GUI fixture tag that deliberately opts into the real persisted startup profile.
pub const GUI_TEST_LIVE_PROFILE_FIXTURE_TAG: &str = "live";

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
            fixture_tag: String::from(GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG),
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
        let mut config = Self {
            run_id: run_id.map(String::from),
            run_manifest_path,
            scenario_name: std::env::var(GUI_TEST_SCENARIO_ENV).ok(),
            fixture_tag: canonical_gui_test_fixture_tag(
                &std::env::var(GUI_TEST_FIXTURE_ENV)
                    .unwrap_or_else(|_| String::from(GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG)),
            )
            .to_owned(),
            ..Self::default()
        };
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

/// Return the canonical GUI fixture tag for configuration and reporting.
///
/// Automated GUI runs use `isolated-startup` as the canonical persisted-startup
/// fixture. The legacy `default` alias remains accepted for compatibility, but
/// it always resolves to the isolated startup profile rather than the live user
/// profile.
pub fn canonical_gui_test_fixture_tag(fixture_tag: &str) -> &str {
    if fixture_tag == LEGACY_DEFAULT_GUI_TEST_FIXTURE_TAG {
        GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG
    } else {
        fixture_tag
    }
}

/// Return whether one GUI fixture tag should load the real persisted startup profile.
pub fn gui_test_fixture_uses_live_profile(fixture_tag: &str) -> bool {
    canonical_gui_test_fixture_tag(fixture_tag) == GUI_TEST_LIVE_PROFILE_FIXTURE_TAG
}

/// Return whether one GUI fixture tag should load isolated persisted startup state.
pub fn gui_test_fixture_uses_isolated_startup(fixture_tag: &str) -> bool {
    canonical_gui_test_fixture_tag(fixture_tag) == GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG
}

fn parse_viewport(value: &str) -> Option<[u32; 2]> {
    let mut parts = value.split('x');
    let width = parts.next()?.trim().parse::<u32>().ok()?;
    let height = parts.next()?.trim().parse::<u32>().ok()?;
    (width > 0 && height > 0).then_some([width, height])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_isolated_startup_fixture() {
        assert_eq!(
            GuiTestModeConfig::default().fixture_tag,
            GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG
        );
    }

    #[test]
    fn legacy_default_fixture_alias_canonicalizes_to_isolated_startup() {
        assert_eq!(
            canonical_gui_test_fixture_tag("default"),
            GUI_TEST_ISOLATED_STARTUP_FIXTURE_TAG
        );
        assert!(gui_test_fixture_uses_isolated_startup("default"));
        assert!(!gui_test_fixture_uses_live_profile("default"));
    }

    #[test]
    fn live_fixture_tag_remains_explicit() {
        assert_eq!(
            canonical_gui_test_fixture_tag(GUI_TEST_LIVE_PROFILE_FIXTURE_TAG),
            GUI_TEST_LIVE_PROFILE_FIXTURE_TAG
        );
        assert!(gui_test_fixture_uses_live_profile(
            GUI_TEST_LIVE_PROFILE_FIXTURE_TAG
        ));
        assert!(!gui_test_fixture_uses_isolated_startup(
            GUI_TEST_LIVE_PROFILE_FIXTURE_TAG
        ));
    }
}
