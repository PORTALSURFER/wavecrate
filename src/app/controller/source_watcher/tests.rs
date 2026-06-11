use super::state::PendingSourceEvent;
use super::*;

#[test]
fn path_is_candidate_filters_db_files() {
    let kind = EventKind::Any;
    assert!(!path_is_candidate(Path::new(DB_FILE_NAME), kind));
    assert!(!path_is_candidate(
        Path::new(&format!("{DB_FILE_NAME}-wal")),
        kind
    ));
    assert!(!path_is_candidate(Path::new(LEGACY_DB_FILE_NAME), kind));
    assert!(!path_is_candidate(
        Path::new(&format!("{LEGACY_DB_FILE_NAME}-wal")),
        kind
    ));
}

#[test]
fn path_is_candidate_allows_supported_audio() {
    let kind = EventKind::Modify(notify::event::ModifyKind::Data(
        notify::event::DataChange::Any,
    ));
    assert!(path_is_candidate(Path::new("kick.wav"), kind));
    assert!(path_is_candidate(Path::new("KICK.WAV"), kind));
    assert!(!path_is_candidate(Path::new("loop.flac"), kind));
}

#[test]
fn path_is_candidate_allows_extensionless_directories() {
    let root = std::env::temp_dir().join("wavecrate_source_watch_dir");
    std::fs::create_dir_all(&root).unwrap();
    assert!(path_is_candidate(&root, EventKind::Any));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn path_is_candidate_allows_removed_extension_named_folders() {
    assert!(path_is_candidate(
        Path::new("Drum.Loops"),
        EventKind::Remove(notify::event::RemoveKind::Folder),
    ));
}

#[test]
fn select_source_for_path_picks_longest_root() {
    let first = SourceWatchEntry::new(SourceId::from_string("a"), PathBuf::from("/music"));
    let second = SourceWatchEntry::new(SourceId::from_string("b"), PathBuf::from("/music/drums"));
    let path = Path::new("/music/drums/kicks/kick.wav");
    let selected = select_source_for_path(&[first, second], path).unwrap();
    assert_eq!(selected.as_str(), "b");
}

#[test]
fn drain_ready_sources_waits_for_debounce() {
    let mut state = SourceWatcherState::default();
    let source_id = SourceId::from_string("a");
    let start = Instant::now();
    state.update_pending_watch(
        source_id.clone(),
        PendingSourceEvent::new(
            SourceWatchCause::ExternalFileChange,
            Some(PathBuf::from("kick.wav")),
        ),
        start,
    );
    assert!(
        state
            .drain_ready_sources(
                start + Duration::from_millis(200),
                Duration::from_millis(400)
            )
            .is_empty()
    );
    let ready = state.drain_ready_sources(
        start + Duration::from_millis(500),
        Duration::from_millis(400),
    );
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].source_id, source_id);
    assert_eq!(ready[0].cause, SourceWatchCause::ExternalFileChange);
    assert_eq!(ready[0].paths, vec![PathBuf::from("kick.wav")]);
    assert!(!ready[0].overflowed);
}

#[test]
fn drain_ready_sources_honors_scan_in_progress() {
    let mut state = SourceWatcherState {
        scan_in_progress: true,
        ..Default::default()
    };
    let source_id = SourceId::from_string("a");
    let start = Instant::now();
    state.update_pending_watch(
        source_id,
        PendingSourceEvent::new(
            SourceWatchCause::ExternalFileChange,
            Some(PathBuf::from("kick.wav")),
        ),
        start,
    );
    let ready = state.drain_ready_sources(
        start + Duration::from_millis(500),
        Duration::from_millis(400),
    );
    assert!(ready.is_empty());
    assert_eq!(state.pending.len(), 1);
}

#[test]
fn controller_owned_path_is_classified_as_controller_file_op() {
    let source = SourceWatchEntry::new(SourceId::from_string("a"), PathBuf::from("/music"));
    let mut controller_file_ops = HashMap::new();
    controller_file_ops.insert(
        source.source_id.clone(),
        HashSet::from([PathBuf::from("drums/kick.wav")]),
    );

    let cause = source_watch_cause_for_path(
        &controller_file_ops,
        &source,
        Path::new("/music/drums/kick.wav"),
    );

    assert_eq!(cause, SourceWatchCause::ControllerFileOp);
}

#[test]
fn unowned_path_during_controller_file_op_falls_back_to_external() {
    let source = SourceWatchEntry::new(SourceId::from_string("a"), PathBuf::from("/music"));
    let mut controller_file_ops = HashMap::new();
    controller_file_ops.insert(
        source.source_id.clone(),
        HashSet::from([PathBuf::from("drums/kick.wav")]),
    );

    let cause = source_watch_cause_for_path(
        &controller_file_ops,
        &source,
        Path::new("/music/drums/snare.wav"),
    );

    assert_eq!(cause, SourceWatchCause::ExternalFileChange);
}

#[test]
fn pending_source_watch_prefers_external_fallback() {
    let mut state = SourceWatcherState::default();
    let source_id = SourceId::from_string("a");
    let start = Instant::now();
    state.update_pending_watch(
        source_id.clone(),
        PendingSourceEvent::new(
            SourceWatchCause::ControllerFileOp,
            Some(PathBuf::from("kick.wav")),
        ),
        start,
    );
    state.update_pending_watch(
        source_id.clone(),
        PendingSourceEvent::new(
            SourceWatchCause::ExternalFileChange,
            Some(PathBuf::from("snare.wav")),
        ),
        start + Duration::from_millis(1),
    );

    let ready = state.drain_ready_sources(
        start + Duration::from_millis(500),
        Duration::from_millis(400),
    );

    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].source_id, source_id);
    assert_eq!(ready[0].cause, SourceWatchCause::ExternalFileChange);
    assert_eq!(
        ready[0].paths,
        vec![PathBuf::from("kick.wav"), PathBuf::from("snare.wav")]
    );
}

#[test]
fn source_watch_event_preserves_relative_paths() {
    let root = PathBuf::from(r"C:\samples");
    let source = SourceWatchEntry::new(SourceId::from_string("a"), root.clone());
    let mut state = SourceWatcherState {
        sources: vec![source],
        ..Default::default()
    };
    let event = Event {
        kind: EventKind::Modify(notify::event::ModifyKind::Data(
            notify::event::DataChange::Any,
        )),
        paths: vec![root.join("drums").join("kick.wav")],
        attrs: Default::default(),
    };

    state.collect_event(event, Instant::now());

    let pending = state.pending.get(&SourceId::from_string("a")).unwrap();
    assert!(pending.paths.contains(&PathBuf::from("drums/kick.wav")));
    assert!(!pending.overflowed);
}

#[test]
fn source_root_event_overflows_to_full_sync() {
    let root = PathBuf::from(r"C:\samples");
    let source = SourceWatchEntry::new(SourceId::from_string("a"), root.clone());
    let mut state = SourceWatcherState {
        sources: vec![source],
        ..Default::default()
    };
    let event = Event {
        kind: EventKind::Any,
        paths: vec![root],
        attrs: Default::default(),
    };

    state.collect_event(event, Instant::now());

    let pending = state.pending.get(&SourceId::from_string("a")).unwrap();
    assert!(pending.paths.is_empty());
    assert!(pending.overflowed);
}
