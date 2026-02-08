//! Migration-facing aliases for native runtime action/model types.
//!
//! These aliases centralize runtime-facing type dependencies in `app_core`,
//! so bridge/controller glue does not import concrete runtime types directly.

/// Native runtime browser action metadata model.
pub type NativeBrowserActionsModel = radiant::app::BrowserActionsModel;

/// Native runtime browser chrome model.
pub type NativeBrowserChromeModel = radiant::app::BrowserChromeModel;

/// Native runtime browser panel model.
pub type NativeBrowserPanelModel = radiant::app::BrowserPanelModel;

/// Native runtime browser row model.
pub type NativeBrowserRowModel = radiant::app::BrowserRowModel;

/// Native runtime UI action payload.
pub type NativeUiAction = radiant::app::UiAction;

/// Native runtime projected app model.
pub type NativeAppModel = radiant::app::AppModel;

/// Native runtime frame build result payload.
pub type NativeFrameBuildResult = radiant::app::FrameBuildResult;

/// Native runtime source row model.
pub type NativeSourceRowModel = radiant::app::SourceRowModel;

/// Native runtime status bar model.
pub type NativeStatusBarModel = radiant::app::StatusBarModel;

/// Native runtime update panel model.
pub type NativeUpdatePanelModel = radiant::app::UpdatePanelModel;

/// Native runtime update status indicator model.
pub type NativeUpdateStatusModel = radiant::app::UpdateStatusModel;

/// Native runtime bridge trait used by host launchers.
pub use radiant::app::NativeAppBridge;
