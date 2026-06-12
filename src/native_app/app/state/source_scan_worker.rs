use std::sync::mpsc::Sender;

use crate::native_app::{
    app::GuiMessage,
    sample_library::folder_browser::scan::{
        self, FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanRequest, FolderScanResult,
    },
};

const DISCOVERY_BATCH_SIZE: usize = 64;

pub(in crate::native_app) fn run_folder_scan_worker(
    request: FolderScanRequest,
    sender: Sender<GuiMessage>,
) -> FolderScanResult {
    let mut discovery_transport = FolderScanDiscoveryTransport::new(
        sender.clone(),
        request.task_id,
        request.source_id.clone(),
    );
    let result = scan::scan_source_with_progress(
        request,
        |progress| {
            let _ = sender.send(GuiMessage::FolderScanProgress(progress));
        },
        |event| {
            discovery_transport.push(event);
        },
    );
    discovery_transport.flush();
    result
}

struct FolderScanDiscoveryTransport {
    sender: Sender<GuiMessage>,
    task_id: u64,
    source_id: String,
    pending: Vec<FolderScanDiscovery>,
}

impl FolderScanDiscoveryTransport {
    fn new(sender: Sender<GuiMessage>, task_id: u64, source_id: String) -> Self {
        Self {
            sender,
            task_id,
            source_id,
            pending: Vec::with_capacity(DISCOVERY_BATCH_SIZE),
        }
    }

    fn push(&mut self, discovery: FolderScanDiscovery) {
        self.pending.push(discovery);
        if self.pending.len() >= DISCOVERY_BATCH_SIZE {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let _ = self.sender.send(GuiMessage::FolderScanDiscoveryBatch(
            FolderScanDiscoveryBatch {
                task_id: self.task_id,
                source_id: self.source_id.clone(),
                events: std::mem::take(&mut self.pending),
            },
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::mpsc};

    use crate::native_app::{
        app::GuiMessage, sample_library::folder_browser::scan::FolderScanRequest,
    };

    use super::{DISCOVERY_BATCH_SIZE, run_folder_scan_worker};

    fn temp_source_with_wavs(count: usize) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("source root");
        for index in 0..count {
            fs::write(
                root.path().join(format!("sample-{index:03}.wav")),
                [0_u8; 8],
            )
            .expect("write wav");
        }
        root
    }

    fn scan_request(root: &tempfile::TempDir) -> FolderScanRequest {
        FolderScanRequest {
            task_id: 42,
            source_id: String::from("source"),
            label: String::from("Source"),
            root: root.path().to_path_buf(),
        }
    }

    #[test]
    fn scan_worker_flushes_discoveries_in_batches() {
        let root = temp_source_with_wavs(DISCOVERY_BATCH_SIZE + 2);
        let (sender, receiver) = mpsc::channel();

        let result = run_folder_scan_worker(scan_request(&root), sender);

        assert_eq!(result.file_count, DISCOVERY_BATCH_SIZE + 2);
        let batch_lengths = receiver
            .try_iter()
            .filter_map(|message| match message {
                GuiMessage::FolderScanDiscoveryBatch(batch) => Some(batch.events.len()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(batch_lengths.first().copied(), Some(DISCOVERY_BATCH_SIZE));
        assert_eq!(
            batch_lengths.iter().sum::<usize>(),
            DISCOVERY_BATCH_SIZE + 3
        );
    }
}
