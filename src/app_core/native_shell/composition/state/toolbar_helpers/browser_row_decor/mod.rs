//! Browser-row indicator, label, and border layout helpers.

use super::super::*;

mod inline_tags;
mod markers;
mod rating_indicators;
mod similarity;

pub(in crate::app_core::native_shell::composition::state) use self::{
    inline_tags::*, markers::*, rating_indicators::*, similarity::*,
};
