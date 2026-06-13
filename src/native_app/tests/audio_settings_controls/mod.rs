use super::gui_state_for_span_tests;
use crate::native_app::test_support::state::NativeAppState;
use radiant::{
    gui::types::Vector2,
    prelude::IntoView,
    widgets::{
        BadgeMessage, BadgeWidget, ButtonMessage, IconButtonWidget, WidgetStyle, WidgetTone,
    },
};
use std::time::{Duration, Instant};

mod routing;
mod runtime_detail;
mod top_bar;
mod volume;
mod window;
