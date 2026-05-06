//! Sempal-owned transitional native Vello runner.
//!
//! This module hosts the legacy native-shell event loop while Sempal migrates
//! toward Radiant's generic `RuntimeBridge` path. It intentionally consumes the
//! local `compat_app_contract` contract instead of Radiant's compatibility facade.

#![allow(dead_code)]

use super::{NativeRunOptions, NativeRuntimeArtifacts, WindowIconRgba};
use crate::gui::{
    input::key_code_from_winit,
    paint::{TextAlign, TextRun},
    types::{Point, Rect as UiRect, Rgba8, Vector2},
};
use skrifa::{
    MetadataProvider,
    instance::{LocationRef, Size as FontSize},
};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::Instant,
};
use tracing::{error, info, warn};
use vello::util::{RenderContext, RenderSurface};
use vello::{
    AaConfig, AaSupport, Glyph, RenderParams, Renderer, RendererOptions, Scene,
    kurbo::{Affine, Rect as KurboRect},
    peniko::{Blob, Color, Fill, FontData, ImageAlphaType, ImageData, ImageFormat},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, Size},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowAttributes, WindowId},
};

mod input;
mod legacy_shell_config;
mod legacy_shell_prelude;
mod legacy_shell_runner;
mod legacy_shell_runtime;
mod legacy_shell_text_entry;
mod profiling;
mod runtime_actions;
mod runtime_config;
mod runtime_event;
mod runtime_events;
mod runtime_input;
mod runtime_render;
mod runtime_startup;
mod runtime_state;
mod scene_cache;
mod scene_rebuild;
mod startup;
mod text_edit;
mod text_renderer;
mod text_runtime;

use self::{
    input::*, legacy_shell_prelude::*, legacy_shell_text_entry::*, profiling::*, runtime_state::*,
    scene_cache::*, scene_rebuild::*, startup::*, text_edit::*, text_renderer::*,
};
pub(in crate::gui_runtime::native_vello) use legacy_shell_config::*;
pub(in crate::gui_runtime::native_vello) use legacy_shell_runner::NativeVelloRunner;
pub(in crate::gui_runtime::native_vello) use runtime_config::*;
pub(in crate::gui_runtime::native_vello) use runtime_event::RuntimeUserEvent;
