/// Maximum consecutive local-only model pulls before forcing one full prep pass.
pub(in crate::app_core::ui_bridge) const LOCAL_MODEL_PULL_FAST_PATH_BURST_LIMIT: u8 = 8;

/// One-shot preparation mode for the next app-model pull.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app_core::ui_bridge) enum PendingModelPullPreparation {
    /// Run the normal full pull-preparation path.
    #[default]
    Full,
    /// Skip full controller prep once and project directly from current UI state.
    LocalOnly,
}
