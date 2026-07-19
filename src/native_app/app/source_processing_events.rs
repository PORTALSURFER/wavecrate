use std::{
    sync::mpsc::Sender,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::native_app::{
    app::{GuiMessage, SourceProcessingProgress},
    source_processing::{
        SourceProcessingActivity, SourceProcessingEvent, SourceProcessingEventSink,
        SourceProcessingProgressEvent,
    },
};

/// Native-shell adapter for backend-neutral source-processing events.
pub(in crate::native_app) struct GuiSourceProcessingEventSink {
    worker_sender: Sender<GuiMessage>,
}

impl GuiSourceProcessingEventSink {
    pub(in crate::native_app) fn new(worker_sender: Sender<GuiMessage>) -> Self {
        Self { worker_sender }
    }
}

impl SourceProcessingEventSink for GuiSourceProcessingEventSink {
    fn try_publish(&self, event: SourceProcessingEvent) -> bool {
        self.worker_sender.send(map_event(event)).is_ok()
    }
}

fn map_event(event: SourceProcessingEvent) -> GuiMessage {
    match event {
        SourceProcessingEvent::Progress(progress) => {
            GuiMessage::SourceProcessingProgress(map_progress(progress))
        }
        SourceProcessingEvent::SimilarityReadinessAdvanced { lifecycle } => {
            GuiMessage::SimilarityReadinessAdvanced {
                source_id: lifecycle.source_id,
                lifecycle_generation: lifecycle.generation,
            }
        }
        SourceProcessingEvent::ManifestAuditCommitted {
            lifecycle,
            committed_delta,
        } => GuiMessage::SourceManifestAuditCommitted {
            source_id: lifecycle.source_id,
            lifecycle_generation: lifecycle.generation,
            committed_delta,
        },
        SourceProcessingEvent::Completed => {
            GuiMessage::SourceProcessingProgress(SourceProcessingProgress {
                source_id: String::new(),
                lifecycle_generation: 0,
                active: false,
                source_row_active: false,
                completed: 0,
                total: 0,
                stage: String::new(),
                detail: String::new(),
            })
        }
    }
}

fn map_progress(event: SourceProcessingProgressEvent) -> SourceProcessingProgress {
    let (stage, detail) = activity_copy(&event.activity);
    SourceProcessingProgress {
        source_id: event.lifecycle.source_id,
        lifecycle_generation: event.lifecycle.generation,
        active: true,
        source_row_active: event.source_row_active,
        completed: event.completed,
        total: event.total,
        stage,
        detail,
    }
}

fn activity_copy(activity: &SourceProcessingActivity) -> (String, String) {
    match activity {
        SourceProcessingActivity::Discovering {
            phase,
            completed_steps,
        } => (
            String::from("Checking pending work"),
            format!("{phase} | {completed_steps} reconciliation steps completed"),
        ),
        SourceProcessingActivity::Readiness {
            stage,
            relative_path,
        } => (
            readiness_stage_label(*stage).to_string(),
            relative_path
                .clone()
                .unwrap_or_else(|| String::from("Finalizing source")),
        ),
        SourceProcessingActivity::ManifestAudit {
            checked,
            relative_path,
        } => (
            String::from("Scanning source changes"),
            match (checked, relative_path) {
                (Some(checked), Some(path)) if !path.as_os_str().is_empty() => {
                    format!("Checked {checked} files | {}", path.display())
                }
                (Some(checked), _) => format!("Resumed after {checked} checked files"),
                (None, _) => String::from("Checking the source manifest"),
            },
        ),
        SourceProcessingActivity::WaitingForPrerequisites { retry_at } => match retry_at {
            Some(retry_at) => {
                let retry_in = retry_at.saturating_sub(now_epoch_seconds());
                (
                    String::from("Waiting for prerequisites"),
                    if retry_in <= 1 {
                        String::from("Prerequisite retry is due")
                    } else {
                        format!("Retrying prerequisites in {retry_in}s")
                    },
                )
            }
            None => (
                String::from("Blocked by prerequisites"),
                String::from("Similarity finalization is waiting for durable prerequisites"),
            ),
        },
        SourceProcessingActivity::WaitingForRetry { retry_at } => {
            let retry_in = retry_at.saturating_sub(now_epoch_seconds());
            (
                String::from("Waiting to retry source"),
                if retry_in <= 1 {
                    String::from("Retry is due")
                } else {
                    format!("A changing or temporarily unavailable file will retry in {retry_in}s")
                },
            )
        }
    }
}

fn readiness_stage_label(
    stage: wavecrate::sample_sources::readiness::ReadinessStage,
) -> &'static str {
    use wavecrate::sample_sources::readiness::ReadinessStage;

    match stage {
        ReadinessStage::IndexedIdentity => "Indexing files",
        ReadinessStage::AnalysisFeatures => "Analyzing audio",
        ReadinessStage::EmbeddingAspects => "Preparing similarity",
        ReadinessStage::SimilarityLayout => "Building similarity layout",
    }
}

fn now_epoch_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::mpsc};

    use wavecrate::sample_sources::readiness::ReadinessStage;

    use super::*;
    use crate::native_app::source_processing::{
        SourceProcessingLifecycle, SourceProcessingProgressEvent,
    };

    fn mapped_progress(activity: SourceProcessingActivity) -> SourceProcessingProgress {
        let (sender, receiver) = mpsc::channel();
        let sink = GuiSourceProcessingEventSink::new(sender);
        assert!(sink.try_publish(SourceProcessingEvent::Progress(
            SourceProcessingProgressEvent {
                lifecycle: SourceProcessingLifecycle::new("source", 17),
                source_row_active: true,
                completed: 3,
                total: 5,
                activity,
            },
        )));
        let GuiMessage::SourceProcessingProgress(progress) =
            receiver.recv().expect("mapped progress")
        else {
            panic!("expected progress message");
        };
        progress
    }

    #[test]
    fn maps_semantic_readiness_and_wait_states_to_existing_gui_copy() {
        let readiness = mapped_progress(SourceProcessingActivity::Readiness {
            stage: ReadinessStage::EmbeddingAspects,
            relative_path: Some(String::from("drums/kick.wav")),
        });
        assert_eq!(readiness.source_id, "source");
        assert_eq!(readiness.lifecycle_generation, 17);
        assert_eq!(readiness.stage, "Preparing similarity");
        assert_eq!(readiness.detail, "drums/kick.wav");

        let blocked =
            mapped_progress(SourceProcessingActivity::WaitingForPrerequisites { retry_at: None });
        assert_eq!(blocked.stage, "Blocked by prerequisites");
        assert_eq!(
            blocked.detail,
            "Similarity finalization is waiting for durable prerequisites"
        );
    }

    #[test]
    fn maps_discovery_audit_and_completion_without_changing_visible_copy() {
        let discovery = mapped_progress(SourceProcessingActivity::Discovering {
            phase: String::from("Comparing durable readiness"),
            completed_steps: 128,
        });
        assert_eq!(discovery.stage, "Checking pending work");
        assert_eq!(
            discovery.detail,
            "Comparing durable readiness | 128 reconciliation steps completed"
        );

        let audit = mapped_progress(SourceProcessingActivity::ManifestAudit {
            checked: Some(9),
            relative_path: Some(PathBuf::from("drums/kick.wav")),
        });
        assert_eq!(audit.stage, "Scanning source changes");
        assert_eq!(audit.detail, "Checked 9 files | drums/kick.wav");

        let GuiMessage::SourceProcessingProgress(completed) =
            map_event(SourceProcessingEvent::Completed)
        else {
            panic!("expected completion progress");
        };
        assert!(!completed.active);
        assert!(completed.source_id.is_empty());
    }

    #[test]
    fn maps_readiness_and_manifest_handoffs_with_their_lifecycle_epoch() {
        let (sender, receiver) = mpsc::channel();
        let sink = GuiSourceProcessingEventSink::new(sender);
        assert!(
            sink.try_publish(SourceProcessingEvent::SimilarityReadinessAdvanced {
                lifecycle: SourceProcessingLifecycle::new("source", 23),
            })
        );
        assert!(
            sink.try_publish(SourceProcessingEvent::ManifestAuditCommitted {
                lifecycle: SourceProcessingLifecycle::new("source", 23),
                committed_delta: wavecrate::sample_sources::scanner::CommittedSourceDelta::default(
                ),
            })
        );

        assert!(matches!(
            receiver.recv().expect("readiness handoff"),
            GuiMessage::SimilarityReadinessAdvanced {
                source_id,
                lifecycle_generation: 23,
            } if source_id == "source"
        ));
        assert!(matches!(
            receiver.recv().expect("manifest handoff"),
            GuiMessage::SourceManifestAuditCommitted {
                source_id,
                lifecycle_generation: 23,
                ..
            } if source_id == "source"
        ));
    }
}
