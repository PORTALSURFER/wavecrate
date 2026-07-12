use super::*;
#[test]
fn startup_source_db_maintenance_defers_same_source_during_file_op() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a"]);
    let source = sources[0].clone();
    controller
        .runtime
        .startup
        .deferred_source_db_maintenance_jobs =
        vec![crate::app::controller::jobs::SourceDbMaintenanceJob {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
        }];
    controller
        .runtime
        .startup
        .deferred_source_db_maintenance_armed = true;
    controller.runtime.startup.frame_prepare_count = 1;

    controller.begin_pending_file_mutation(&source.id, [PathBuf::from("alpha.wav")]);
    controller.flush_deferred_startup_source_db_maintenance();

    assert!(controller.has_pending_startup_source_db_maintenance());
    assert!(!controller.runtime.jobs.source_db_maintenance_in_progress());
    assert_eq!(
        controller
            .runtime
            .startup
            .deferred_source_db_maintenance_jobs
            .iter()
            .map(|job| job.source_id.clone())
            .collect::<Vec<_>>(),
        vec![source.id.clone()]
    );

    controller.finish_pending_file_mutation(&source.id, [PathBuf::from("alpha.wav")]);
    controller.flush_deferred_startup_source_db_maintenance();

    assert!(!controller.has_pending_startup_source_db_maintenance());
}

#[test]
fn startup_source_db_maintenance_defers_same_source_during_live_remap() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a"]);
    let source = sources[0].clone();
    controller
        .runtime
        .startup
        .deferred_source_db_maintenance_jobs =
        vec![crate::app::controller::jobs::SourceDbMaintenanceJob {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
        }];
    controller
        .runtime
        .startup
        .deferred_source_db_maintenance_armed = true;
    controller.runtime.startup.frame_prepare_count = 1;
    let remapped_root = tempfile::tempdir().expect("remap destination").keep();
    controller.runtime.source_lane.pending_remap =
        Some(crate::app::controller::state::runtime::PendingSourceRemap {
            request_id: 92,
            source: source.clone(),
            new_root: remapped_root.clone(),
            queued_at: std::time::Instant::now(),
            canceled: false,
        });

    controller.flush_deferred_startup_source_db_maintenance();

    assert!(controller.has_pending_startup_source_db_maintenance());
    assert!(!controller.runtime.jobs.source_db_maintenance_in_progress());
    assert_eq!(
        controller
            .runtime
            .startup
            .deferred_source_db_maintenance_jobs[0]
            .source_id,
        source.id
    );

    controller.library.sources[0].root = remapped_root.clone();
    controller.runtime.source_lane.pending_remap = None;
    controller.begin_pending_file_mutation(&source.id, [PathBuf::from("alpha.wav")]);
    controller.flush_deferred_startup_source_db_maintenance();

    assert_eq!(
        controller
            .runtime
            .startup
            .deferred_source_db_maintenance_jobs[0]
            .source_root,
        remapped_root
    );
    controller.finish_pending_file_mutation(&source.id, [PathBuf::from("alpha.wav")]);
}

#[test]
fn startup_source_db_maintenance_allows_other_sources_during_file_op() {
    let (mut controller, sources) = build_controller_with_sources(&["source-a", "source-b"]);
    controller
        .runtime
        .startup
        .deferred_source_db_maintenance_jobs = sources
        .iter()
        .map(
            |source| crate::app::controller::jobs::SourceDbMaintenanceJob {
                source_id: source.id.clone(),
                source_root: source.root.clone(),
            },
        )
        .collect();
    controller
        .runtime
        .startup
        .deferred_source_db_maintenance_armed = true;
    controller.runtime.startup.frame_prepare_count = 1;

    controller.begin_pending_file_mutation(&sources[0].id, [PathBuf::from("alpha.wav")]);
    controller.flush_deferred_startup_source_db_maintenance();

    assert!(controller.has_pending_startup_source_db_maintenance());
    assert_eq!(
        controller
            .runtime
            .startup
            .deferred_source_db_maintenance_jobs
            .iter()
            .map(|job| job.source_id.clone())
            .collect::<Vec<_>>(),
        vec![sources[0].id.clone()]
    );
    controller.finish_pending_file_mutation(&sources[0].id, [PathBuf::from("alpha.wav")]);
}
