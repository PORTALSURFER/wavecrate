//! Focused browser helper modules for normalization, focus planning, and sample mutations.

use super::*;
use crate::app::controller::jobs::{
    FileOpProgressSender, FileOpResult, NormalizationJob, SampleAutoRenameResult,
    SampleAutoRenameSuccess, SampleRenameResult,
};
use crate::app::controller::state::runtime::BrowserRenameIntentKey;
use crate::app::controller::undo;
use std::sync::{Arc, atomic::AtomicBool};

mod controller;
mod focus;
mod normalization;
mod sample_mutation;
#[cfg(test)]
mod sample_mutation_tests;

pub(crate) use controller::{BrowserController, DeleteBrowserFocusPlan, TriageSampleContext};
pub(crate) use sample_mutation::{SampleAutoRenameRequest, run_sample_auto_rename_job};
