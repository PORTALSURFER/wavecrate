use super::super::super::test_support::{
    load_waveform_selection, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::jobs::JobMessage;
use crate::app::controller::library::analysis_jobs::AnalysisJobMessage;
use crate::app::controller::library::selection_edits::SelectionEditRequest;
use crate::app::state::{DestructiveSelectionEdit, WaveformView};
use crate::app_core::state::StatusTone;
use crate::app_core::ui_projection::project_browser_model;
use crate::sample_sources::SampleSoundType;
use crate::selection::SelectionRange;
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::path::Path;
use std::time::{Duration, Instant};

mod align_start;
mod duplicate_cleanup;
mod export_follow_up;
mod in_place_operations;
mod metadata_cache;
mod safety_prompt;
