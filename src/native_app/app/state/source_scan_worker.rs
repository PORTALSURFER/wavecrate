use radiant::prelude as ui;

use crate::native_app::sample_library::folder_browser::scan::{
    self, FolderScanDiscovery, FolderScanDiscoveryBatch, FolderScanProgress, FolderScanRequest,
    FolderScanResult,
};

const DISCOVERY_BATCH_SIZE: usize = 64;

pub(in crate::native_app) enum FolderScanWorkerEvent {
    Progress(FolderScanProgress),
    DiscoveryBatch(FolderScanDiscoveryBatch),
}

pub(in crate::native_app) fn run_folder_scan_worker(
    request: FolderScanRequest,
    events: ui::BusinessEventSink<FolderScanWorkerEvent>,
) -> FolderScanResult {
    run_folder_scan_worker_with_emit(request, move |event| events.emit(event))
}

fn run_folder_scan_worker_with_emit(
    request: FolderScanRequest,
    emit: impl Fn(FolderScanWorkerEvent) -> bool + Clone,
) -> FolderScanResult {
    let mut discovery_transport =
        FolderScanDiscoveryTransport::new(emit.clone(), request.task_id, request.source_id.clone());
    let result = scan::scan_source_with_progress(
        request,
        |progress| {
            let _ = emit(FolderScanWorkerEvent::Progress(progress));
        },
        |event| {
            discovery_transport.push(event);
        },
    );
    discovery_transport.flush();
    result
}

struct FolderScanDiscoveryTransport<Emit> {
    emit: Emit,
    task_id: u64,
    source_id: String,
    pending: Vec<FolderScanDiscovery>,
}

impl<Emit> FolderScanDiscoveryTransport<Emit>
where
    Emit: Fn(FolderScanWorkerEvent) -> bool,
{
    fn new(emit: Emit, task_id: u64, source_id: String) -> Self {
        Self {
            emit,
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
        let _ = (self.emit)(FolderScanWorkerEvent::DiscoveryBatch(
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

    use crate::native_app::sample_library::folder_browser::scan::FolderScanRequest;

    use super::{DISCOVERY_BATCH_SIZE, FolderScanWorkerEvent, run_folder_scan_worker_with_emit};

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
            database_root: root.path().to_path_buf(),
            rating_decay_weeks: FolderScanRequest::default_rating_decay_weeks(),
        }
    }

    #[test]
    fn scan_worker_flushes_discoveries_in_batches() {
        let root = temp_source_with_wavs(DISCOVERY_BATCH_SIZE + 2);
        let (sender, receiver) = mpsc::channel();

        let result = run_folder_scan_worker_with_emit(scan_request(&root), move |event| {
            sender.send(event).is_ok()
        });

        assert_eq!(result.file_count, DISCOVERY_BATCH_SIZE + 2);
        let batch_lengths = receiver
            .try_iter()
            .filter_map(|message| match message {
                FolderScanWorkerEvent::DiscoveryBatch(batch) => Some(batch.events.len()),
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
