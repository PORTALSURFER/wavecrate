//! Installer workflow state and worker-thread coordination.

use std::{path::PathBuf, sync::mpsc, thread};

use crate::{install, paths};

/// Events emitted by installer worker threads and consumed by the UI bridge.
pub(crate) enum InstallerEvent {
    Started { total_files: usize },
    FileCopied { copied_files: usize, name: String },
    Log(String),
    Finished,
    Failed(String),
}

/// Shared sender type used by installer tasks and helper modules.
pub(crate) type InstallerSender = mpsc::Sender<InstallerEvent>;

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum InstallStep {
    Welcome,
    License,
    Location,
    Installing,
    Done,
    Error,
}

#[derive(Default)]
pub(crate) struct InstallProgress {
    pub(crate) total_files: usize,
    pub(crate) copied_files: usize,
    pub(crate) current: Option<String>,
}

pub(crate) struct InstallerNativeBridge {
    pub(crate) step: InstallStep,
    pub(crate) install_dir: PathBuf,
    pub(crate) bundle_dir: PathBuf,
    pub(crate) license_text: String,
    pub(crate) progress: InstallProgress,
    receiver: Option<mpsc::Receiver<InstallerEvent>>,
    pub(crate) error: Option<String>,
    pub(crate) open_folder_on_finish: bool,
    pub(crate) launch_on_finish: bool,
    pub(crate) finish_errors: Vec<String>,
    pub(crate) logs: Vec<String>,
    pub(crate) install_finished: bool,
}

impl InstallerNativeBridge {
    pub(crate) fn new() -> Self {
        Self {
            step: InstallStep::Welcome,
            install_dir: paths::default_install_dir(),
            bundle_dir: paths::default_bundle_dir(),
            license_text: include_str!("../../../../LICENSE").to_string(),
            progress: InstallProgress::default(),
            receiver: None,
            error: None,
            open_folder_on_finish: true,
            launch_on_finish: true,
            finish_errors: Vec::new(),
            logs: Vec::new(),
            install_finished: false,
        }
    }

    pub(crate) fn start_install(&mut self) {
        let bundle_dir = self.bundle_dir.clone();
        let install_dir = self.install_dir.clone();
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        self.progress = InstallProgress::default();
        self.step = InstallStep::Installing;
        self.install_finished = false;
        self.finish_errors.clear();
        self.logs.clear();
        thread::spawn(move || {
            if let Err(err) = install::run_install(&bundle_dir, &install_dir, tx.clone()) {
                let _ = tx.send(InstallerEvent::Failed(err));
            }
        });
    }

    pub(crate) fn poll_installer(&mut self) {
        let Some(receiver) = &self.receiver else {
            return;
        };
        while let Ok(event) = receiver.try_recv() {
            match event {
                InstallerEvent::Started { total_files } => {
                    self.progress.total_files = total_files;
                }
                InstallerEvent::FileCopied { copied_files, name } => {
                    self.progress.copied_files = copied_files;
                    self.progress.current = Some(name);
                }
                InstallerEvent::Log(message) => {
                    self.logs.push(message);
                }
                InstallerEvent::Finished => {
                    self.install_finished = true;
                    self.step = InstallStep::Done;
                }
                InstallerEvent::Failed(err) => {
                    self.error = Some(err);
                    self.step = InstallStep::Error;
                }
            }
        }
    }

    pub(crate) fn browse_install_dir(&mut self) {
        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
            self.install_dir = folder;
        }
    }

    pub(crate) fn advance_step(&mut self) {
        match self.step {
            InstallStep::Welcome => self.step = InstallStep::License,
            InstallStep::License => self.step = InstallStep::Location,
            InstallStep::Location => self.start_install(),
            InstallStep::Done => self.run_finish_actions(),
            InstallStep::Error => self.start_install(),
            InstallStep::Installing => {}
        }
    }

    pub(crate) fn back_step(&mut self) {
        match self.step {
            InstallStep::Welcome => super::request_process_exit(),
            InstallStep::License => self.step = InstallStep::Welcome,
            InstallStep::Location => self.step = InstallStep::License,
            InstallStep::Done => super::request_process_exit(),
            InstallStep::Error => super::request_process_exit(),
            InstallStep::Installing => {}
        }
    }

    fn run_finish_actions(&mut self) {
        self.finish_errors.clear();
        if self.open_folder_on_finish
            && let Err(err) = open::that(&self.install_dir)
        {
            self.finish_errors
                .push(format!("Failed to open install folder: {err}"));
        }
        if self.launch_on_finish {
            let exe = self.install_dir.join("sempal.exe");
            if let Err(err) = std::process::Command::new(exe).spawn() {
                self.finish_errors
                    .push(format!("Failed to launch SemPal: {err}"));
            }
        }
        if self.finish_errors.is_empty() {
            super::request_process_exit();
        }
    }
}
