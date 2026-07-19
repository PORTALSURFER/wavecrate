use super::*;
use crate::app::controller::state::cache::{FeatureCache, FeatureCacheKey};
use crate::app::controller::test_support::dummy_controller;
use crate::app_core::state::StatusTone;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn reconciliation_keeps_selected_cache_and_queues_refresh() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.ui_cache.browser.features.insert(
        source.id.clone(),
        FeatureCache {
            key: FeatureCacheKey::default(),
            rows: Vec::new().into(),
        },
    );

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::ReadinessReconciliationFinished {
            source_id: source.id.clone(),
            changed: 2,
            announce: true,
        },
    );

    assert_eq!(
        controller.ui.status.text,
        "Queued readiness reconciliation for 2 samples"
    );
    assert!(
        controller
            .ui_cache
            .browser
            .features
            .contains_key(&source.id)
    );
    assert!(
        controller
            .runtime
            .browser
            .pending_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| pending.source_id == source.id)
    );
}

#[test]
fn quiet_reconciliation_preserves_existing_status() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.set_status("Saved clip clip_selection_001.wav", StatusTone::Info);

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::ReadinessReconciliationFinished {
            source_id: source.id,
            changed: 1,
            announce: false,
        },
    );

    assert_eq!(
        controller.ui.status.text,
        "Saved clip clip_selection_001.wav"
    );
}

#[test]
fn durations_updated_keeps_selected_feature_cache_and_queues_refresh() {
    let (mut controller, source) = dummy_controller();
    let source_id = source.id.clone();
    controller.library.sources.push(source);
    controller.ui_cache.browser.features.insert(
        source_id.clone(),
        FeatureCache {
            key: FeatureCacheKey::default(),
            rows: Vec::new().into(),
        },
    );
    controller.ui_cache.browser.durations.insert(
        source_id.clone(),
        HashMap::from([(PathBuf::from("kick.wav"), 1.25)]),
    );

    handle_analysis_message(
        &mut controller,
        AnalysisJobMessage::DurationsUpdated {
            source_id: source_id.clone(),
            updated: 1,
        },
    );

    assert!(
        controller
            .ui_cache
            .browser
            .features
            .contains_key(&source_id)
    );
    assert!(
        controller
            .runtime
            .browser
            .pending_feature_cache_refresh
            .as_ref()
            .is_some_and(|pending| pending.source_id == source_id)
    );
    assert!(
        !controller
            .ui_cache
            .browser
            .durations
            .contains_key(&source_id)
    );
}
