use super::*;

fn source_id() -> SourceId {
    SourceId::from_string("source")
}

fn candidate(path: &str, index: usize) -> BrowserSelectionCandidate {
    BrowserSelectionCandidate::new(source_id(), PathBuf::from(path), index)
}

fn commit_request(load_requested: bool) -> BrowserSelectionCommitRequest {
    BrowserSelectionCommitRequest {
        record_focus_history: true,
        refresh_similarity_highlight: true,
        load_requested,
    }
}

#[test]
fn preview_candidate_preserves_existing_matching_load_state() {
    let initial = BrowserSelectionTransitionMachine::new(None)
        .note_loading(
            candidate("kick.wav", 3),
            BrowserSelectionCommitStage::Preview,
            None,
        )
        .transition;

    let outcome =
        BrowserSelectionTransitionMachine::new(initial).publish_preview(candidate("kick.wav", 3));

    let transition = outcome.transition.expect("transition");
    assert!(matches!(
        transition.commit_stage,
        BrowserSelectionCommitStage::Preview
    ));
    assert_eq!(transition.load_state, BrowserSelectionLoadState::Loading);
}

#[test]
fn commit_with_load_queues_history_similarity_and_audio_load() {
    let outcome = BrowserSelectionTransitionMachine::new(None).publish_commit(
        candidate("kick.wav", 3),
        commit_request(true),
        None,
    );
    let transition = outcome.transition.expect("transition");

    let outcome = BrowserSelectionTransitionMachine::new(Some(transition)).flush_commit(true);

    assert!(matches!(
        outcome
            .transition
            .as_ref()
            .map(|transition| &transition.commit_stage),
        Some(BrowserSelectionCommitStage::Settled)
    ));
    assert!(outcome.effects.iter().any(|effect| matches!(
        effect,
        BrowserSelectionTransitionEffect::RecordFocusHistory { .. }
    )));
    assert!(outcome.effects.iter().any(|effect| matches!(
        effect,
        BrowserSelectionTransitionEffect::RefreshSimilarityHighlight(_)
    )));
    assert!(
        outcome
            .effects
            .iter()
            .any(|effect| matches!(effect, BrowserSelectionTransitionEffect::QueueAudioLoad(_)))
    );
}

#[test]
fn stale_commit_cancels_load_and_drops_transition() {
    let outcome = BrowserSelectionTransitionMachine::new(None).publish_commit(
        candidate("kick.wav", 3),
        commit_request(true),
        None,
    );
    let transition = outcome.transition.expect("transition");

    let outcome = BrowserSelectionTransitionMachine::new(Some(transition)).flush_commit(false);

    assert!(outcome.transition.is_none());
    assert!(matches!(
        outcome.effects.as_slice(),
        [BrowserSelectionTransitionEffect::CancelLoad(_)]
    ));
}

#[test]
fn awaiting_waveform_updates_matching_loading_candidate() {
    let initial = BrowserSelectionTransitionMachine::new(None)
        .note_loading(
            candidate("kick.wav", 3),
            BrowserSelectionCommitStage::Settled,
            None,
        )
        .transition;

    let outcome = BrowserSelectionTransitionMachine::new(initial)
        .note_awaiting_waveform(&source_id(), Path::new("kick.wav"));

    assert_eq!(
        outcome.transition.expect("transition").load_state,
        BrowserSelectionLoadState::AwaitingWaveform
    );
}

#[test]
fn superseded_pending_commit_records_previous_history() {
    let initial = BrowserSelectionTransitionMachine::new(None)
        .publish_commit(candidate("kick.wav", 3), commit_request(false), None)
        .transition;

    let outcome = BrowserSelectionTransitionMachine::new(initial).publish_commit(
        candidate("snare.wav", 4),
        commit_request(false),
        None,
    );

    assert!(outcome.effects.iter().any(|effect| matches!(
        effect,
        BrowserSelectionTransitionEffect::RecordFocusHistory { relative_path, .. }
            if relative_path == Path::new("kick.wav")
    )));
    assert_eq!(
        outcome.transition.expect("transition").relative_path,
        PathBuf::from("snare.wav")
    );
}
