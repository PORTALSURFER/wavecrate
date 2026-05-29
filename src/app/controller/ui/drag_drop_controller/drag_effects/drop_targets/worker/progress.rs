use std::sync::mpsc::Sender;

use crate::app::controller::jobs::FileOpMessage;

pub(super) struct DropTargetTransferProgress<'a> {
    sender: Option<&'a Sender<FileOpMessage>>,
    completed: usize,
}

impl<'a> DropTargetTransferProgress<'a> {
    pub(super) fn new(sender: Option<&'a Sender<FileOpMessage>>) -> Self {
        Self {
            sender,
            completed: 0,
        }
    }

    pub(super) fn complete(&mut self, detail: Option<String>) {
        self.completed = self.completed.saturating_add(1);
        if let Some(tx) = self.sender {
            let _ = tx.send(FileOpMessage::Progress {
                completed: self.completed,
                detail,
                item: None,
            });
        }
    }
}
