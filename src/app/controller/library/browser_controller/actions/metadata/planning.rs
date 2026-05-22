use super::*;

mod capture;
mod controller;
mod logging;
mod target;
mod worker;

#[derive(Clone)]
pub(super) struct AutoRenameBackgroundRequest {
    pub(super) source: SampleSource,
    pub(super) paths: Vec<PathBuf>,
    pub(super) identifier: String,
    pub(super) is_playing: bool,
    pub(super) resume_looped: bool,
    pub(super) resume_start_override: Option<f64>,
    pub(super) loaded_relative: Option<PathBuf>,
    pub(super) metadata: HashMap<PathBuf, AutoRenamePathMetadata>,
}

#[derive(Clone, Default)]
pub(super) struct AutoRenamePathMetadata {
    pub(super) entry: Option<WavEntry>,
    pub(super) normal_tags: Option<Vec<String>>,
    pub(super) bpm: Option<f32>,
}

pub(super) use worker::run_background_auto_rename_request;
