use super::classification::path_is_source_refresh_candidate;
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
