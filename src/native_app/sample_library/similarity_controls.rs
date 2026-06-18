use std::time::{Duration, Instant};

use radiant::prelude as ui;
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore, SimilarityAspectSettings};
use wavecrate_analysis::aspects::SimilarityAspect;

use crate::native_app::app::{
    GuiMessage, NativeAppState, SimilaritySettingsPersistResult, emit_gui_action,
};

const SIMILARITY_SETTINGS_PERSIST_DEBOUNCE: Duration = Duration::from_millis(250);

impl NativeAppState {
    pub(in crate::native_app) fn set_similarity_aspect_weighting_enabled(&mut self, enabled: bool) {
        let mut controls = self.library.folder_browser.similarity_controls().clone();
        controls.set_weighting_enabled(enabled);
        self.apply_similarity_controls(controls);
    }

    pub(in crate::native_app) fn set_similarity_aspect_enabled(
        &mut self,
        aspect: SimilarityAspect,
        enabled: bool,
    ) {
        let mut controls = self.library.folder_browser.similarity_controls().clone();
        controls.set_aspect_enabled(aspect, enabled);
        self.apply_similarity_controls(controls);
    }

    pub(in crate::native_app) fn set_similarity_aspect_weight(
        &mut self,
        aspect: SimilarityAspect,
        weight: f32,
    ) {
        let mut controls = self.library.folder_browser.similarity_controls().clone();
        controls.set_aspect_weight(aspect, weight);
        self.apply_similarity_controls(controls);
    }

    pub(in crate::native_app) fn flush_pending_similarity_settings_persist(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(deadline) = self.ui.settings.similarity_persist_deadline else {
            return;
        };
        if Instant::now() < deadline || self.ui.settings.similarity_persist_inflight {
            return;
        }
        self.ui.settings.similarity_persist_deadline = None;
        self.ui.settings.similarity_persist_inflight = true;

        let persisted = self.current_settings_core();
        let sources = self.library.folder_browser.configured_sample_sources();
        context
            .business()
            .blocking_io("gui-similarity-settings-persist")
            .run(
                move |_| persist_similarity_settings(sources, persisted),
                GuiMessage::SimilaritySettingsPersisted,
            );
    }

    pub(in crate::native_app) fn finish_similarity_settings_persist(
        &mut self,
        result: SimilaritySettingsPersistResult,
    ) {
        let started_at = Instant::now();
        self.ui.settings.similarity_persist_inflight = false;
        match result.result {
            Ok(()) => {
                self.ui.settings.persisted = result.persisted;
                emit_gui_action(
                    "browser.similarity.controls.persist",
                    Some("folder_browser"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = format!("Similarity settings not saved: {error}");
                emit_gui_action(
                    "browser.similarity.controls.persist",
                    Some("folder_browser"),
                    None,
                    "persist_error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    fn apply_similarity_controls(&mut self, controls: SimilarityAspectSettings) {
        self.library
            .folder_browser
            .set_similarity_controls(controls);
        self.ui.settings.similarity_persist_deadline =
            Some(Instant::now() + SIMILARITY_SETTINGS_PERSIST_DEBOUNCE);
    }
}

fn persist_similarity_settings(
    sources: Vec<wavecrate::sample_sources::SampleSource>,
    persisted: AppSettingsCore,
) -> SimilaritySettingsPersistResult {
    let result = wavecrate::sample_sources::config::save(&AppConfig {
        sources,
        core: persisted.clone(),
    })
    .map_err(|err| err.to_string());
    SimilaritySettingsPersistResult { persisted, result }
}
