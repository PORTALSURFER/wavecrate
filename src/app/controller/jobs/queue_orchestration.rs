use super::{FileOpMessage, JobMessage, ScanJobMessage};
use crate::app::controller::library::{analysis_jobs::AnalysisJobMessage, trash_move};
use std::sync::mpsc::{Receiver, SendError, SyncSender, TrySendError, sync_channel};

/// Bounded sender for job messages with best-effort delivery for low-priority updates.
#[derive(Clone)]
pub(crate) struct JobMessageSender {
    inner: SyncSender<JobMessage>,
}

impl JobMessageSender {
    /// Create a sender that writes into the controller's bounded job queue.
    pub(crate) fn new(inner: SyncSender<JobMessage>) -> Self {
        Self { inner }
    }

    /// Send a job message, dropping low-priority updates if the queue is full.
    pub(crate) fn send(&self, message: JobMessage) -> Result<(), SendError<JobMessage>> {
        match job_message_delivery(&message) {
            JobMessageDelivery::MustDeliver => self.inner.send(message),
            JobMessageDelivery::DropIfFull => match self.inner.try_send(message) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_)) => Ok(()),
                Err(TrySendError::Disconnected(message)) => Err(SendError(message)),
            },
        }
    }
}

/// Delivery policy for a single job message when queue pressure is present.
enum JobMessageDelivery {
    MustDeliver,
    DropIfFull,
}

/// Resolve the delivery strategy for one queued controller job message.
fn job_message_delivery(message: &JobMessage) -> JobMessageDelivery {
    match message {
        JobMessage::Scan(ScanJobMessage::Progress { .. }) => JobMessageDelivery::DropIfFull,
        JobMessage::TrashMove(trash_move::TrashMoveMessage::Progress { .. }) => {
            JobMessageDelivery::DropIfFull
        }
        JobMessage::FileOps(FileOpMessage::Progress { .. }) => JobMessageDelivery::DropIfFull,
        JobMessage::Analysis(AnalysisJobMessage::Progress { .. }) => JobMessageDelivery::DropIfFull,
        _ => JobMessageDelivery::MustDeliver,
    }
}

/// Create the bounded controller job queue.
pub(super) fn new_job_message_queue(capacity: usize) -> (JobMessageSender, Receiver<JobMessage>) {
    let bounded_capacity = capacity.max(1);
    let (tx, rx) = sync_channel::<JobMessage>(bounded_capacity);
    (JobMessageSender::new(tx), rx)
}
