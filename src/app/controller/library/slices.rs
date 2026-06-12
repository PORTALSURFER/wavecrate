use super::AppController;
use crate::app::controller::StatusTone;
use crate::app::state::WaveformSliceBatchProfile;
use crate::selection::SelectionRange;

mod collection;
mod detection;
mod duplicate_preview;
mod exact_duplicates;
mod export;
mod ops;
mod review;
mod selection;

#[cfg(test)]
mod ops_tests;
#[cfg(test)]
mod slices_tests;
