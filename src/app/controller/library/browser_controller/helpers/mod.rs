//! Focused browser helper modules for normalization, focus planning, and sample mutations.

use super::*;
use crate::app::controller::jobs::{
    FileOpResult, NormalizationJob, SampleAutoRenameResult, SampleAutoRenameSuccess,
    SampleRenameResult,
};
use crate::app::controller::undo;
use std::sync::{atomic::AtomicBool, Arc};

mod controller;
mod focus;
mod normalization;
mod sample_mutation;
#[cfg(test)]
mod sample_mutation_tests;

pub(crate) use controller::{BrowserController, DeleteBrowserFocusPlan, TriageSampleContext};
pub(crate) use sample_mutation::{run_sample_auto_rename_job, SampleAutoRenameRequest};
