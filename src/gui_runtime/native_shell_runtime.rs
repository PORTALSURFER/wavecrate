//! Deprecated Wavecrate native-shell compatibility adapter for Radiant's generic
//! runtime path.
//!
//! This module owns the focused Wavecrate runtime adapter modules that project the
//! legacy app model into Radiant's generic native runtime, route retained-shell
//! input, convert local action/model shapes, and expose automation/snapshot
//! helpers. Normal GUI execution should go through the default `src/gui_app.rs`
//! Radiant application path.

use super::{NativeRunOptions, NativeRunReport, NativeRuntimeArtifacts, WindowIconRgba};
use crate::app_core::actions::{
    NativeAppBridge, NativeAppModel, NativeBrowserTagTarget as BrowserTagTarget,
    NativeGuiAutomationSnapshot, NativeMotionModel, NativeUiAction as UiAction,
    native_shell_dtos::*,
};
use crate::app_core::app_api::controller_ui_hotkeys::KeyPress;
use crate::app_core::app_api::{controller_ui_hotkeys as hotkeys, state::FocusContext};
use crate::app_core::native_shell::composition::{
    NativeShellState, ShellLayout, ShellLayoutRuntime, ShellNodeKind, StaticFrameSegment,
    StaticFrameSegments, StyleTokens,
};
use crate::app_core::native_shell::runtime_contract;
use crate::gui::automation as gui_automation;
use crate::gui::{
    paint::PaintFrame,
    types::{Point, Rect, Vector2},
};
use radiant::gui::{
    focus::FocusSurface as RadiantFocusSurface,
    input::{KeyCode as RadiantKeyCode, KeyPress as RadiantKeyPress},
    shortcuts::ShortcutResolution as RadiantShortcutResolution,
};
use radiant::runtime::{Command, RuntimeBridge, SurfaceNode, UiSurface};
use radiant::widgets::{
    CanvasMessage, PointerButton, RetainedSurfaceDescriptor, TextEditCommand, WidgetInput,
    WidgetKey, WidgetSizing,
};
use std::{collections::BTreeMap, sync::Arc};

mod action_mapping;
mod automation;
mod bridge;
mod input_routing;
mod launch;
mod model_mapping;

pub(super) use automation::capture_gui_automation_snapshot;
#[cfg(test)]
pub(super) use automation::capture_native_shell_shot_snapshot;
use bridge::WavecrateRuntimeBridge;
#[cfg(test)]
use bridge::WavecrateRuntimeMessage;
use input_routing::{
    action_from_retained_pointer, keypress_from_radiant, keypress_to_radiant,
    wavecrate_focus_context,
};
pub(super) use launch::{run_native_vello_app, run_native_vello_app_with_artifacts};
use model_mapping::local_app_model_from_native_model;

#[cfg(test)]
mod tests;
