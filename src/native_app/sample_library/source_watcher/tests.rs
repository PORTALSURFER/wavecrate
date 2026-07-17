use super::classification::{path_is_source_refresh_candidate, retain_source_refresh_candidates};
use super::handle::doubled_backoff;
use super::roots::RootWatchUpdate;
use super::state::GuiSourceWatchState;
use notify::{Event, EventKind, event::RemoveKind};
use std::{path::PathBuf, time::Instant};
use wavecrate::sample_sources::{SampleSource, SourceId};

#[test]
fn removed_extension_named_folder_triggers_source_refresh() {
    let root = PathBuf::from(r"C:\samples");
    let source =
        SampleSource::new_with_id(SourceId::from_string("source_id::samples"), root.clone());
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let event = Event {
        kind: EventKind::Remove(RemoveKind::Folder),
        paths: vec![root.join("Drum.Loops")],
        attrs: Default::default(),
    };

    state.collect_event(&event, Instant::now());

    let pending = state.pending.get("source_id::samples").unwrap();
    assert!(pending.paths.contains(&PathBuf::from("Drum.Loops")));
    assert!(!pending.overflowed);
}

#[test]
fn wavecrate_metadata_files_do_not_trigger_source_refresh() {
    let root = PathBuf::from(r"C:\samples");
    assert!(!path_is_source_refresh_candidate(
        &root.join(wavecrate::sample_sources::db::DB_FILE_NAME),
        EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any
        )),
    ));
}

#[test]
fn metadata_event_storm_is_filtered_before_the_bounded_watcher_queue() {
    let root = PathBuf::from(r"C:\samples");
    let mut event = Event {
        kind: EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any,
        )),
        paths: vec![
            root.join(wavecrate::sample_sources::db::DB_FILE_NAME),
            root.join(format!(
                "{}-wal",
                wavecrate::sample_sources::db::DB_FILE_NAME
            )),
            root.join("kick.wav"),
        ],
        attrs: Default::default(),
    };

    assert!(retain_source_refresh_candidates(&mut event));
    assert_eq!(event.paths, vec![root.join("kick.wav")]);

    event.paths = vec![root.join(wavecrate::sample_sources::db::DB_FILE_NAME)];
    assert!(!retain_source_refresh_candidates(&mut event));
    assert!(event.paths.is_empty());
}

#[test]
fn apple_double_sidecars_do_not_trigger_source_refresh() {
    let root = PathBuf::from(r"C:\samples");
    assert!(!path_is_source_refresh_candidate(
        &root.join("._kick.wav"),
        EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any
        )),
    ));
    assert!(!path_is_source_refresh_candidate(
        &root.join("drums").join("._snare.wav"),
        EventKind::Create(notify::event::CreateKind::File),
    ));
}

#[test]
fn transient_ann_persistence_artifacts_do_not_trigger_source_refresh() {
    let root = PathBuf::from(r"C:\samples");
    for path in [
        root.join("ann_containerMqsFy3"),
        root.join("ann_dumpJgi2JJ"),
        root.join("ann_dumpJgi2JJ").join("ann_dump.hnsw.data"),
        root.join("ann_dumpJgi2JJ").join("ann_dump.hnsw.graph"),
    ] {
        assert!(!path_is_source_refresh_candidate(
            &path,
            EventKind::Create(notify::event::CreateKind::Any),
        ));
    }
    assert!(path_is_source_refresh_candidate(
        &root.join("ann_dump.wav"),
        EventKind::Create(notify::event::CreateKind::File),
    ));
    assert!(path_is_source_refresh_candidate(
        &root.join("ann_dump_samples"),
        EventKind::Create(notify::event::CreateKind::Folder),
    ));
}

#[test]
fn source_root_event_overflows_to_full_refresh() {
    let root = PathBuf::from(r"C:\samples");
    let source =
        SampleSource::new_with_id(SourceId::from_string("source_id::samples"), root.clone());
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let event = Event {
        kind: EventKind::Any,
        paths: vec![root],
        attrs: Default::default(),
    };

    state.collect_event(&event, Instant::now());

    let pending = state.pending.get("source_id::samples").unwrap();
    assert!(pending.paths.is_empty());
    assert!(pending.overflowed);
}

#[test]
fn debounce_drain_waits_until_source_is_ready() {
    let root = PathBuf::from(r"C:\samples");
    let source =
        SampleSource::new_with_id(SourceId::from_string("source_id::samples"), root.clone());
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let started = Instant::now();
    let event = Event {
        kind: EventKind::Any,
        paths: vec![root.join("kick.wav")],
        attrs: Default::default(),
    };
    state.collect_event(&event, started);

    assert!(
        state
            .drain_ready_sources(started, super::SOURCE_CHANGE_DEBOUNCE)
            .is_empty()
    );

    let ready = state.drain_ready_sources(
        started + super::SOURCE_CHANGE_DEBOUNCE,
        super::SOURCE_CHANGE_DEBOUNCE,
    );

    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].source_id, "source_id::samples");
    assert_eq!(ready[0].paths, vec![PathBuf::from("kick.wav")]);
    assert!(!ready[0].overflowed);
}

#[test]
fn debounce_drain_reports_root_availability_from_the_watcher_thread() {
    let root = tempfile::tempdir().expect("source root");
    let source = SampleSource::new_with_id(
        SourceId::from_string("source_id::availability"),
        root.path().to_path_buf(),
    );
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let started = Instant::now();

    state.mark_all_overflowed(started);
    let ready = state.drain_ready_sources(
        started + super::SOURCE_CHANGE_DEBOUNCE,
        super::SOURCE_CHANGE_DEBOUNCE,
    );
    assert!(ready[0].source_root_available);

    std::fs::remove_dir_all(root.path()).expect("remove source root");
    state.mark_all_overflowed(started + super::SOURCE_CHANGE_DEBOUNCE);
    let ready = state.drain_ready_sources(
        started + super::SOURCE_CHANGE_DEBOUNCE * 2,
        super::SOURCE_CHANGE_DEBOUNCE,
    );
    assert!(!ready[0].source_root_available);
}

#[test]
fn live_root_events_do_not_feed_database_writes_back_into_scans() {
    let root = tempfile::tempdir().expect("source root");
    let source = SampleSource::new_with_id(
        SourceId::from_string("source_id::root-event"),
        root.path().to_path_buf(),
    );
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };

    state.collect_event(
        &Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Any),
            paths: vec![root.path().to_path_buf()],
            attrs: Default::default(),
        },
        Instant::now(),
    );

    assert!(state.pending.is_empty());
}

#[test]
fn initial_watch_registration_does_not_queue_full_source_refresh() {
    let root = tempfile::tempdir().expect("source root");
    let source = SampleSource::new_with_id(
        SourceId::from_string("source_id::initial-watch"),
        root.path().to_path_buf(),
    );
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let (_unavailable, failed) = state.apply_root_watch_update(
        RootWatchUpdate {
            changed_roots: vec![root.path().to_path_buf()],
            has_unavailable_roots: false,
            watch_failed: false,
        },
        Instant::now(),
        false,
    );

    assert!(!failed);
    assert!(state.pending.is_empty());
}

#[test]
fn later_watch_registration_queues_authoritative_source_refresh() {
    let root = tempfile::tempdir().expect("source root");
    let source = SampleSource::new_with_id(
        SourceId::from_string("source_id::reappeared-watch"),
        root.path().to_path_buf(),
    );
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let (_unavailable, failed) = state.apply_root_watch_update(
        RootWatchUpdate {
            changed_roots: vec![root.path().to_path_buf()],
            has_unavailable_roots: false,
            watch_failed: false,
        },
        Instant::now(),
        true,
    );

    assert!(!failed);
    assert!(
        state
            .pending
            .get("source_id::reappeared-watch")
            .is_some_and(|pending| pending.overflowed)
    );
}

#[test]
fn path_churn_is_bounded_and_escalates_to_full_refresh() {
    let root = PathBuf::from(r"C:\samples");
    let source =
        SampleSource::new_with_id(SourceId::from_string("source_id::samples"), root.clone());
    let mut state = GuiSourceWatchState {
        sources: vec![source],
        ..Default::default()
    };
    let started = Instant::now();
    for index in 0..=super::MAX_PENDING_PATHS_PER_SOURCE {
        state.collect_event(
            &Event {
                kind: EventKind::Create(notify::event::CreateKind::File),
                paths: vec![root.join(format!("sample-{index}.wav"))],
                attrs: Default::default(),
            },
            started,
        );
    }

    let pending = state.pending.get("source_id::samples").unwrap();
    assert!(pending.overflowed);
    assert!(pending.paths.is_empty());
}

#[test]
fn watcher_restart_marks_every_source_for_authoritative_refresh() {
    let first = SampleSource::new_with_id(
        SourceId::from_string("source_id::first"),
        PathBuf::from(r"C:\first"),
    );
    let second = SampleSource::new_with_id(
        SourceId::from_string("source_id::second"),
        PathBuf::from(r"C:\second"),
    );
    let mut state = GuiSourceWatchState {
        sources: vec![first, second],
        ..Default::default()
    };
    let started = Instant::now();

    state.mark_all_overflowed(started);
    let mut ready = state.drain_ready_sources(
        started + super::SOURCE_CHANGE_DEBOUNCE,
        super::SOURCE_CHANGE_DEBOUNCE,
    );
    ready.sort_by(|left, right| left.source_id.cmp(&right.source_id));

    assert_eq!(ready.len(), 2);
    assert!(ready.iter().all(|event| event.overflowed));
    assert!(ready.iter().all(|event| event.paths.is_empty()));
}

#[test]
fn watcher_restart_backoff_is_bounded() {
    assert_eq!(
        doubled_backoff(super::WATCHER_RESTART_MIN),
        super::WATCHER_RESTART_MIN * 2
    );
    assert_eq!(
        doubled_backoff(super::WATCHER_RESTART_MAX),
        super::WATCHER_RESTART_MAX
    );
}
