use super::{
    ControllerUiCacheState, LibraryCacheState, MissingState, SourceDatabase, SourceId,
    controller_state::FeatureCache,
};
use crate::app::controller::state::cache::FolderBrowserCacheKey;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    rc::Rc,
};

pub(crate) struct SourceCacheInvalidator<'a> {
    db_cache: &'a mut HashMap<SourceId, Rc<SourceDatabase>>,
    wav_cache: &'a mut HashMap<SourceId, super::WavEntriesState>,
    label_cache: &'a mut HashMap<SourceId, Vec<String>>,
    bpm_cache: &'a mut HashMap<SourceId, HashMap<PathBuf, Option<f32>>>,
    duration_cache: &'a mut HashMap<SourceId, HashMap<PathBuf, f32>>,
    analysis_failures_cache: &'a mut HashMap<SourceId, HashMap<PathBuf, String>>,
    feature_cache: &'a mut HashMap<SourceId, FeatureCache>,
    browser_pipeline_cache: &'a mut crate::app::controller::library::wavs::BrowserPipelineCache,
    missing_wavs: &'a mut HashMap<SourceId, HashSet<PathBuf>>,
    folder_browsers: &'a mut HashMap<
        FolderBrowserCacheKey,
        crate::app::controller::library::source_folders::FolderBrowserModel,
    >,
}

impl<'a> SourceCacheInvalidator<'a> {
    pub(crate) fn new_from_state(
        cache: &'a mut LibraryCacheState,
        ui_cache: &'a mut ControllerUiCacheState,
        missing: &'a mut MissingState,
    ) -> Self {
        Self {
            db_cache: &mut cache.db,
            wav_cache: &mut cache.wav.entries,
            label_cache: &mut ui_cache.browser.labels,
            bpm_cache: &mut ui_cache.browser.bpm_values,
            duration_cache: &mut ui_cache.browser.durations,
            analysis_failures_cache: &mut ui_cache.browser.analysis_failures,
            feature_cache: &mut ui_cache.browser.features,
            browser_pipeline_cache: &mut ui_cache.browser.pipeline,
            missing_wavs: &mut missing.wavs,
            folder_browsers: &mut ui_cache.folders.models,
        }
    }

    pub(crate) fn invalidate_wav_related(&mut self, source_id: &SourceId) {
        self.wav_cache.remove(source_id);
        self.label_cache.remove(source_id);
        self.bpm_cache.remove(source_id);
        self.duration_cache.remove(source_id);
        self.analysis_failures_cache.remove(source_id);
        self.feature_cache.remove(source_id);
        self.browser_pipeline_cache.invalidate();
        self.missing_wavs.remove(source_id);
    }

    pub(crate) fn invalidate_db_cache(&mut self, source_id: &SourceId) {
        self.db_cache.remove(source_id);
    }

    pub(crate) fn invalidate_folder_browser(&mut self, source_id: &SourceId) {
        self.folder_browsers
            .retain(|key, _| &key.source_id != source_id);
    }

    pub(crate) fn invalidate_all(&mut self, source_id: &SourceId) {
        self.invalidate_db_cache(source_id);
        self.invalidate_wav_related(source_id);
        self.invalidate_folder_browser(source_id);
    }
}
