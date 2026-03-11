//! GUI test contracts, deterministic artifact helpers, and scenario runners.

mod artifacts;
mod config;
mod runner;
mod scenario;

pub use self::artifacts::{
    GuiActionTraceEvent, GuiModelSummary, GuiStepTimingSample, GuiTestArtifactBundle,
    build_model_summary, write_artifact_bundle,
};
pub use self::config::GuiTestModeConfig;
pub use self::runner::{
    capture_default_bundle, dispatch_action_bundle, export_aiv_suite, run_scenario,
};
pub use self::scenario::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(crate) use self::artifacts::{catalog_report, trace_event_for_action};
