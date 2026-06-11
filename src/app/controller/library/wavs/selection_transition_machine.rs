use std::path::{Path, PathBuf};

use crate::app::controller::state::audio::PendingPlayback;
use crate::app::controller::state::runtime::{
    BrowserSelectionCommitRequest, BrowserSelectionCommitStage, BrowserSelectionLoadState,
    BrowserSelectionTransition,
};
use crate::sample_sources::SourceId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserSelectionCandidate {
    pub(super) source_id: SourceId,
    pub(super) relative_path: PathBuf,
    pub(super) entry_index: usize,
}

impl BrowserSelectionCandidate {
    pub(super) fn new(source_id: SourceId, relative_path: PathBuf, entry_index: usize) -> Self {
        Self {
            source_id,
            relative_path,
            entry_index,
        }
    }

    fn matches_transition(&self, transition: &BrowserSelectionTransition) -> bool {
        self.source_id == transition.source_id
            && self.relative_path == transition.relative_path
            && self.entry_index == transition.entry_index
    }
}

#[derive(Clone, Debug)]
pub(super) enum BrowserSelectionTransitionEffect {
    RecordFocusHistory {
        source_id: SourceId,
        relative_path: PathBuf,
    },
    RefreshSimilarityHighlight(BrowserSelectionTransition),
    CancelLoad(BrowserSelectionTransition),
    QueueAudioLoad(BrowserSelectionTransition),
}

#[derive(Clone, Debug, Default)]
pub(super) struct BrowserSelectionTransitionOutcome {
    pub(super) transition: Option<BrowserSelectionTransition>,
    pub(super) effects: Vec<BrowserSelectionTransitionEffect>,
}

pub(super) struct BrowserSelectionTransitionMachine {
    transition: Option<BrowserSelectionTransition>,
}

impl BrowserSelectionTransitionMachine {
    pub(super) fn new(transition: Option<BrowserSelectionTransition>) -> Self {
        Self { transition }
    }

    pub(super) fn publish_preview(
        self,
        candidate: BrowserSelectionCandidate,
    ) -> BrowserSelectionTransitionOutcome {
        let mut next = transition_for_candidate(
            candidate,
            BrowserSelectionCommitStage::Preview,
            BrowserSelectionLoadState::Stable,
            None,
        );
        if let Some(current) = self
            .transition
            .as_ref()
            .filter(|current| candidate_matches_transition(&next, current))
        {
            next.commit_stage = current.commit_stage.clone();
            next.load_state = current.load_state;
            next.pending_playback = current.pending_playback.clone();
        }
        self.replace(next)
    }

    pub(super) fn publish_commit(
        self,
        candidate: BrowserSelectionCandidate,
        request: BrowserSelectionCommitRequest,
        pending_playback: Option<PendingPlayback>,
    ) -> BrowserSelectionTransitionOutcome {
        let mut next = transition_for_candidate(
            candidate,
            BrowserSelectionCommitStage::DispatchPending(request),
            BrowserSelectionLoadState::Stable,
            pending_playback,
        );
        if let Some(current) = self
            .transition
            .as_ref()
            .filter(|current| candidate_matches_transition(&next, current))
        {
            next.load_state = current.load_state;
        }
        self.replace(next)
    }

    pub(super) fn note_loading(
        self,
        candidate: BrowserSelectionCandidate,
        fallback_stage: BrowserSelectionCommitStage,
        pending_playback: Option<PendingPlayback>,
    ) -> BrowserSelectionTransitionOutcome {
        let mut next = self
            .transition
            .clone()
            .filter(|current| candidate.matches_transition(current))
            .unwrap_or_else(|| {
                transition_for_candidate(
                    candidate,
                    fallback_stage,
                    BrowserSelectionLoadState::Stable,
                    None,
                )
            });
        next.load_state = BrowserSelectionLoadState::Loading;
        next.pending_playback = pending_playback;
        self.replace(next)
    }

    pub(super) fn note_awaiting_waveform(
        mut self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> BrowserSelectionTransitionOutcome {
        if let Some(transition) = self.transition.as_mut().filter(|transition| {
            &transition.source_id == source_id
                && transition.relative_path.as_path() == relative_path
        }) {
            transition.load_state = BrowserSelectionLoadState::AwaitingWaveform;
        }
        BrowserSelectionTransitionOutcome {
            transition: self.transition,
            effects: Vec::new(),
        }
    }

    pub(super) fn clear(
        self,
        source_id: &SourceId,
        relative_path: &Path,
    ) -> BrowserSelectionTransitionOutcome {
        let transition = self.transition.filter(|transition| {
            &transition.source_id != source_id
                || transition.relative_path.as_path() != relative_path
        });
        BrowserSelectionTransitionOutcome {
            transition,
            effects: Vec::new(),
        }
    }

    pub(super) fn flush_commit(
        self,
        candidate_is_current: bool,
    ) -> BrowserSelectionTransitionOutcome {
        let Some(mut transition) = self.transition else {
            return BrowserSelectionTransitionOutcome::default();
        };
        let BrowserSelectionCommitStage::DispatchPending(request) = transition.commit_stage.clone()
        else {
            return BrowserSelectionTransitionOutcome {
                transition: Some(transition),
                effects: Vec::new(),
            };
        };
        if !candidate_is_current {
            let effects = transition
                .load_requested()
                .then(|| BrowserSelectionTransitionEffect::CancelLoad(transition));
            return BrowserSelectionTransitionOutcome {
                transition: None,
                effects: effects.into_iter().collect(),
            };
        }

        let mut effects = commit_effects(&transition, &request);
        transition.commit_stage = BrowserSelectionCommitStage::Settled;
        if request.load_requested && transition.load_state == BrowserSelectionLoadState::Stable {
            effects.push(BrowserSelectionTransitionEffect::QueueAudioLoad(
                transition.clone(),
            ));
        }
        BrowserSelectionTransitionOutcome {
            transition: Some(transition),
            effects,
        }
    }

    pub(super) fn abandon_for_navigation(self) -> BrowserSelectionTransitionOutcome {
        let Some(mut transition) = self.transition else {
            return BrowserSelectionTransitionOutcome::default();
        };
        let BrowserSelectionCommitStage::DispatchPending(request) = transition.commit_stage.clone()
        else {
            return BrowserSelectionTransitionOutcome {
                transition: Some(transition),
                effects: Vec::new(),
            };
        };
        let effects = request.record_focus_history.then(|| {
            BrowserSelectionTransitionEffect::RecordFocusHistory {
                source_id: transition.source_id.clone(),
                relative_path: transition.relative_path.clone(),
            }
        });
        transition.commit_stage = BrowserSelectionCommitStage::Settled;
        let transition =
            (transition.load_state != BrowserSelectionLoadState::Stable).then_some(transition);
        BrowserSelectionTransitionOutcome {
            transition,
            effects: effects.into_iter().collect(),
        }
    }

    pub(super) fn replace_transition(
        self,
        next: BrowserSelectionTransition,
    ) -> BrowserSelectionTransitionOutcome {
        self.replace(next)
    }

    fn replace(self, next: BrowserSelectionTransition) -> BrowserSelectionTransitionOutcome {
        let mut effects = Vec::new();
        if let Some(previous) = self.transition
            && !candidate_matches_transition(&previous, &next)
            && matches!(
                next.commit_stage,
                BrowserSelectionCommitStage::DispatchPending(_)
            )
            && let BrowserSelectionCommitStage::DispatchPending(request) = previous.commit_stage
            && request.record_focus_history
        {
            effects.push(BrowserSelectionTransitionEffect::RecordFocusHistory {
                source_id: previous.source_id,
                relative_path: previous.relative_path,
            });
        }
        BrowserSelectionTransitionOutcome {
            transition: Some(next),
            effects,
        }
    }
}

fn transition_for_candidate(
    candidate: BrowserSelectionCandidate,
    commit_stage: BrowserSelectionCommitStage,
    load_state: BrowserSelectionLoadState,
    pending_playback: Option<PendingPlayback>,
) -> BrowserSelectionTransition {
    BrowserSelectionTransition {
        source_id: candidate.source_id,
        relative_path: candidate.relative_path,
        entry_index: candidate.entry_index,
        commit_stage,
        load_state,
        pending_playback,
    }
}

fn candidate_matches_transition(
    left: &BrowserSelectionTransition,
    right: &BrowserSelectionTransition,
) -> bool {
    left.source_id == right.source_id
        && left.relative_path == right.relative_path
        && left.entry_index == right.entry_index
}

fn commit_effects(
    transition: &BrowserSelectionTransition,
    request: &BrowserSelectionCommitRequest,
) -> Vec<BrowserSelectionTransitionEffect> {
    let mut effects = Vec::new();
    if request.record_focus_history {
        effects.push(BrowserSelectionTransitionEffect::RecordFocusHistory {
            source_id: transition.source_id.clone(),
            relative_path: transition.relative_path.clone(),
        });
    }
    if request.refresh_similarity_highlight {
        effects
            .push(BrowserSelectionTransitionEffect::RefreshSimilarityHighlight(transition.clone()));
    }
    effects
}

#[cfg(test)]
mod tests;
