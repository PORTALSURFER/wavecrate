use super::*;
use crate::app::controller::jobs::{FileOpMessage, FileOpResult, FolderCreateResult};
use std::fs;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool};

use super::planning::{FolderCreateCommand, plan_folder_create};

impl AppController {
    pub(crate) fn create_folder(&mut self, parent: &Path, name: &str) -> Result<(), String> {
        let command = plan_folder_create(self, parent, name)?;
        self.launch_folder_create(command);
        Ok(())
    }

    fn launch_folder_create(&mut self, command: FolderCreateCommand) {
        self.begin_pending_file_mutation(&command.source.id, [command.relative.clone()]);
        if cfg!(test) {
            self.apply_synchronous_folder_create(command);
            return;
        }
        self.set_status(
            format!("Creating folder {}...", command.relative.display()),
            StatusTone::Busy,
        );
        self.spawn_folder_create_job(command);
    }

    fn apply_synchronous_folder_create(&mut self, command: FolderCreateCommand) {
        let result = FolderCreateResult {
            source_id: command.source.id,
            relative_path: command.relative,
            result: fs::create_dir_all(&command.destination)
                .map_err(|err| format!("Failed to create folder: {err}")),
        };
        self.apply_file_op_result(FileOpResult::FolderCreate(result));
    }

    fn spawn_folder_create_job(&mut self, command: FolderCreateCommand) {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                Err(String::from("Folder creation cancelled"))
            } else {
                fs::create_dir_all(&command.destination)
                    .map_err(|err| format!("Failed to create folder: {err}"))
            };
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::FolderCreate(
                FolderCreateResult {
                    source_id: command.source.id,
                    relative_path: command.relative,
                    result,
                },
            )));
        });
    }
}
