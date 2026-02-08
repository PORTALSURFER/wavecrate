//! Migration-facing aliases for native runtime action/model types.
//!
//! These aliases centralize runtime-facing type dependencies in `app_core`,
//! so bridge/controller glue does not import concrete runtime types directly.

/// Native runtime UI action payload.
pub type NativeUiAction = radiant::app::UiAction;

/// Native runtime projected app model.
pub type NativeAppModel = radiant::app::AppModel;

/// Native runtime frame build result payload.
pub type NativeFrameBuildResult = radiant::app::FrameBuildResult;
