/// Background worker implementation for non-blocking selection exports.
mod background;
/// Worker-side entry registration helpers for background selection exports.
mod background_recording;
/// Controller-facing selection-export command entrypoints.
mod commands;
/// UI-thread completion handlers for finished selection exports.
mod completion;
/// UI feedback and timing helpers for selection exports.
mod feedback;
/// Helper routines shared by the selection-export workflow.
mod helpers;
/// Staged worker-side selection export pipelines.
mod pipeline;
/// Persistence and cache-registration steps for newly exported clips.
mod recording;
/// Typed export/registration request objects used by the selection-export workflow.
mod requests;
/// Waveform slice-batch export orchestration.
mod slice_batch;
/// Controller-side selection-export snapshot and planning helpers.
mod snapshot;
/// Synchronous selection clip export helpers used by drag/clipboard paths.
mod sync_clip;

pub(crate) use self::background::run_selection_export_job;
pub(crate) use self::background::run_slice_batch_export_job;
pub(crate) use self::helpers::cleanup_written_export_after_registration_failure as cleanup_unregistered_source_export;
pub(crate) use self::requests::{SelectionClipExportRequest, SelectionEntryRecordRequest};

use super::selection_edits::apply_short_edge_fades_to_clip;
use super::*;
use crate::app::controller::jobs::{
    SelectionClipDestination, SelectionClipExportSuccess, SelectionCropExportSuccess,
    SelectionExportJob, SelectionExportSnapshot, SelectionExportTimings,
    build_selection_export_audio_payload,
};
use crate::app::controller::playback::audio_samples::write_wav_with_spec;
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

impl AppController {
    /// Queue one selection export while retaining source-write ownership until completion.
    pub(crate) fn queue_selection_export_job(&mut self, job: SelectionExportJob) {
        let source_id = job.destination_source_id().clone();
        self.cancel_pending_source_remap_for_mutation(&source_id);
        self.runtime.jobs.begin_selection_export(job);
    }

    /// Queue one slice batch while retaining source-write ownership until completion.
    pub(crate) fn queue_selection_slice_batch_export_job(&mut self, job: SelectionExportJob) {
        let source_id = job.destination_source_id().clone();
        self.cancel_pending_source_remap_for_mutation(&source_id);
        self.runtime.jobs.begin_selection_slice_batch_export(job);
    }
}

#[cfg(test)]
mod selection_export_tests;
