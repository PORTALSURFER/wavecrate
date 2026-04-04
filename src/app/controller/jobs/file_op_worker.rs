//! Reusable worker lane for one-shot file operations that only emit a final result.

use super::*;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::JoinHandle;

type FileOpTask = Box<dyn FnOnce(Arc<AtomicBool>) -> FileOpResult + Send + 'static>;

/// One queued file-operation task executed by the reusable worker thread.
pub(super) struct QueuedFileOpTask {
    cancel: Arc<AtomicBool>,
    run: FileOpTask,
}

impl QueuedFileOpTask {
    /// Capture one background file operation and its cooperative cancel handle.
    pub(super) fn new(
        cancel: Arc<AtomicBool>,
        run: impl FnOnce(Arc<AtomicBool>) -> FileOpResult + Send + 'static,
    ) -> Self {
        Self {
            cancel,
            run: Box::new(run),
        }
    }
}

/// Handle for the dedicated one-shot file-operation worker.
pub(super) struct FileOpWorkerHandle {
    sender: Option<Sender<QueuedFileOpTask>>,
    join: Option<JoinHandle<()>>,
}

impl FileOpWorkerHandle {
    /// Spawn the reusable file-operation worker and bind it to the controller job queue.
    pub(super) fn spawn(
        message_tx: JobMessageSender,
        repaint_signal: Arc<SharedRepaintSignal>,
    ) -> Self {
        let (sender, rx) = mpsc::channel();
        let join = thread::spawn(move || run_file_op_worker(rx, message_tx, repaint_signal));
        Self {
            sender: Some(sender),
            join: Some(join),
        }
    }

    /// Queue one file-operation task onto the reusable worker.
    pub(super) fn send(&self, task: QueuedFileOpTask) -> Result<(), String> {
        self.sender
            .as_ref()
            .ok_or_else(|| "File operation worker is unavailable".to_string())?
            .send(task)
            .map_err(|_| "Failed to queue file operation".to_string())
    }

    /// Shut the worker down and join the backing thread.
    pub(super) fn shutdown(&mut self) {
        self.sender.take();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run_file_op_worker(
    rx: Receiver<QueuedFileOpTask>,
    message_tx: JobMessageSender,
    repaint_signal: Arc<SharedRepaintSignal>,
) {
    while let Ok(task) = rx.recv() {
        let result = (task.run)(task.cancel);
        let _ = message_tx.send(JobMessage::FileOps(FileOpMessage::Finished(result)));
        repaint_signal.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    #[test]
    fn reusable_file_op_worker_delivers_finished_messages() {
        let (tx, rx) = sync_channel(4);
        let worker =
            FileOpWorkerHandle::spawn(JobMessageSender::new(tx), Arc::new(SharedRepaintSignal::default()));
        let source_id = SourceId::from_string("source");

        worker
            .send(QueuedFileOpTask::new(
                Arc::new(AtomicBool::new(false)),
                move |_| {
                    FileOpResult::FolderCreate(FolderCreateResult {
                        source_id: source_id.clone(),
                        relative_path: PathBuf::from("folder"),
                        result: Ok(()),
                    })
                },
            ))
            .expect("queue file op");

        match rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker result")
        {
            JobMessage::FileOps(FileOpMessage::Finished(FileOpResult::FolderCreate(result))) => {
                assert_eq!(result.relative_path, PathBuf::from("folder"));
                assert!(result.result.is_ok());
            }
            other => panic!("unexpected file-op worker message: {other:?}"),
        }
    }
}
