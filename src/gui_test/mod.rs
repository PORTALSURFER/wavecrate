//! GUI test contracts, deterministic artifact helpers, and scenario runners.

mod automation;
mod artifacts;
mod config;
mod fixtures;
mod packs;
mod runner;
mod scenario;

pub use self::automation::{
    GuiAutomationTarget, read_automation_snapshot_from_artifact, resolve_automation_target,
};
pub use self::artifacts::{
    GuiActionTraceEvent, GuiModelSummary, GuiStepTimingSample, GuiTestArtifactBundle,
    build_model_summary, write_artifact_bundle,
};
pub use self::config::GuiTestModeConfig;
pub use self::fixtures::GuiFixtureBridge;
pub use self::packs::{GuiScenarioPack, gui_scenario_pack};
pub use self::runner::{
    capture_default_bundle, dispatch_action_bundle, export_aiv_suite, run_scenario,
};
pub use self::scenario::{GuiAssertion, GuiScenario, GuiScenarioStep};

pub(crate) use self::artifacts::{catalog_report, trace_event_for_action};
pub(crate) use self::automation::find_automation_node;
