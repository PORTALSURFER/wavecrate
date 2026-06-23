//! Windows installer entry point for Wavecrate.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use std::{env, process::ExitCode};

mod cleanup;
mod download;
mod events;
mod install;
mod paths;
mod registry;
mod shortcuts;

const APP_NAME: &str = "SemPal";
#[cfg(target_os = "windows")]
const APP_PUBLISHER: &str = "SemPal";
#[cfg(target_os = "windows")]
const UNINSTALL_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SemPal";

fn main() -> ExitCode {
    match run_with_args(
        env::args(),
        cleanup::run_uninstall,
        install::run_dry_run,
        run_headless_install,
        events::removed_interactive_installer_entrypoint,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallerEntryCommand {
    Uninstall,
    DryRun,
    Install,
    RemovedInteractive,
}

fn run_with_args<I, U, D, H, L>(
    args: I,
    run_uninstall: U,
    run_dry_run: D,
    run_install: H,
    run_removed_interactive: L,
) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
    U: FnOnce() -> Result<(), String>,
    D: FnOnce() -> Result<(), String>,
    H: FnOnce() -> Result<(), String>,
    L: FnOnce() -> Result<(), String>,
{
    match select_entry_command(args) {
        InstallerEntryCommand::Uninstall => {
            run_uninstall().map_err(|err| format!("Uninstall failed: {err}"))
        }
        InstallerEntryCommand::DryRun => {
            run_dry_run().map_err(|err| format!("Dry run failed: {err}"))
        }
        InstallerEntryCommand::Install => {
            run_install().map_err(|err| format!("Install failed: {err}"))
        }
        InstallerEntryCommand::RemovedInteractive => run_removed_interactive(),
    }
}

fn select_entry_command<I>(args: I) -> InstallerEntryCommand
where
    I: IntoIterator<Item = String>,
{
    let mut command = InstallerEntryCommand::RemovedInteractive;
    for arg in args.into_iter().skip(1) {
        if arg == "--uninstall" {
            return InstallerEntryCommand::Uninstall;
        }
        if arg == "--dry-run" {
            command = InstallerEntryCommand::DryRun;
        }
        if arg == "--install" {
            command = InstallerEntryCommand::Install;
        }
    }
    command
}

fn run_headless_install() -> Result<(), String> {
    let bundle_dir = paths::default_bundle_dir();
    let install_dir = paths::default_install_dir();
    let (sender, receiver) = std::sync::mpsc::channel();
    let result = install::run_install(&bundle_dir, &install_dir, sender);
    for event in receiver.try_iter() {
        match event {
            events::InstallerEvent::Started { total_files } => {
                println!("Installing {total_files} files");
            }
            events::InstallerEvent::FileCopied { copied_files, name } => {
                println!("Copied {copied_files}: {name}");
            }
            events::InstallerEvent::Log(message) => println!("{message}"),
            events::InstallerEvent::Finished => println!("Install complete"),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::install::{PlanAction, plan_install};
    use super::{InstallerEntryCommand, run_with_args, select_entry_command};
    use std::cell::Cell;
    use std::fs;

    #[test]
    fn dry_run_plans_bundle_copies() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = temp.path().join("bundle");
        let install = temp.path().join("install");
        fs::create_dir_all(bundle.join("bin")).expect("bundle dir");
        fs::write(bundle.join("bin").join("wavecrate.exe"), "test").expect("exe");

        let plan = plan_install(&bundle, &install).expect("plan");
        let copies = plan
            .actions
            .iter()
            .filter(|action| matches!(action, PlanAction::Copy { .. }))
            .count();
        assert_eq!(copies, 1);
    }

    #[test]
    fn select_entry_command_defaults_to_removed_interactive_entrypoint() {
        let command = select_entry_command(vec![String::from("wavecrate-installer")]);
        assert_eq!(command, InstallerEntryCommand::RemovedInteractive);
    }

    #[test]
    fn select_entry_command_uses_dry_run_when_requested() {
        let command = select_entry_command(vec![
            String::from("wavecrate-installer"),
            String::from("--dry-run"),
        ]);
        assert_eq!(command, InstallerEntryCommand::DryRun);
    }

    #[test]
    fn select_entry_command_uses_install_when_requested() {
        let command = select_entry_command(vec![
            String::from("wavecrate-installer"),
            String::from("--install"),
        ]);
        assert_eq!(command, InstallerEntryCommand::Install);
    }

    #[test]
    fn select_entry_command_prefers_uninstall_over_dry_run() {
        let command = select_entry_command(vec![
            String::from("wavecrate-installer"),
            String::from("--dry-run"),
            String::from("--uninstall"),
        ]);
        assert_eq!(command, InstallerEntryCommand::Uninstall);
    }

    #[test]
    fn run_with_args_dispatches_uninstall_without_removed_interactive_entrypoint() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let install_called = Cell::new(false);
        let removed_interactive_called = Cell::new(false);

        let result = run_with_args(
            vec![
                String::from("wavecrate-installer"),
                String::from("--uninstall"),
            ],
            || {
                uninstall_called.set(true);
                Ok(())
            },
            || {
                dry_run_called.set(true);
                Ok(())
            },
            || {
                install_called.set(true);
                Ok(())
            },
            || {
                removed_interactive_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(uninstall_called.get());
        assert!(!dry_run_called.get());
        assert!(!install_called.get());
        assert!(!removed_interactive_called.get());
    }

    #[test]
    fn run_with_args_dispatches_dry_run_without_removed_interactive_entrypoint() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let install_called = Cell::new(false);
        let removed_interactive_called = Cell::new(false);

        let result = run_with_args(
            vec![
                String::from("wavecrate-installer"),
                String::from("--dry-run"),
            ],
            || {
                uninstall_called.set(true);
                Ok(())
            },
            || {
                dry_run_called.set(true);
                Ok(())
            },
            || {
                install_called.set(true);
                Ok(())
            },
            || {
                removed_interactive_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(!uninstall_called.get());
        assert!(dry_run_called.get());
        assert!(!install_called.get());
        assert!(!removed_interactive_called.get());
    }

    #[test]
    fn run_with_args_propagates_dry_run_errors() {
        let result = run_with_args(
            vec![
                String::from("wavecrate-installer"),
                String::from("--dry-run"),
            ],
            || Ok(()),
            || Err(String::from("bundle missing")),
            || Ok(()),
            || Ok(()),
        );

        assert_eq!(result, Err(String::from("Dry run failed: bundle missing")));
    }

    #[test]
    fn run_with_args_propagates_uninstall_errors() {
        let result = run_with_args(
            vec![
                String::from("wavecrate-installer"),
                String::from("--uninstall"),
            ],
            || Err(String::from("registry unavailable")),
            || Ok(()),
            || Ok(()),
            || Ok(()),
        );

        assert_eq!(
            result,
            Err(String::from("Uninstall failed: registry unavailable"))
        );
    }

    #[test]
    fn run_with_args_dispatches_headless_install_without_removed_interactive_entrypoint() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let install_called = Cell::new(false);
        let removed_interactive_called = Cell::new(false);

        let result = run_with_args(
            vec![
                String::from("wavecrate-installer"),
                String::from("--install"),
            ],
            || {
                uninstall_called.set(true);
                Ok(())
            },
            || {
                dry_run_called.set(true);
                Ok(())
            },
            || {
                install_called.set(true);
                Ok(())
            },
            || {
                removed_interactive_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(!uninstall_called.get());
        assert!(!dry_run_called.get());
        assert!(install_called.get());
        assert!(!removed_interactive_called.get());
    }

    #[test]
    fn run_with_args_uses_removed_interactive_entrypoint_by_default() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let install_called = Cell::new(false);
        let removed_interactive_called = Cell::new(false);

        let result = run_with_args(
            vec![String::from("wavecrate-installer")],
            || {
                uninstall_called.set(true);
                Ok(())
            },
            || {
                dry_run_called.set(true);
                Ok(())
            },
            || {
                install_called.set(true);
                Ok(())
            },
            || {
                removed_interactive_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(!uninstall_called.get());
        assert!(!dry_run_called.get());
        assert!(!install_called.get());
        assert!(removed_interactive_called.get());
    }

    #[test]
    fn run_with_args_propagates_removed_interactive_entrypoint_errors() {
        let result = run_with_args(
            vec![String::from("wavecrate-installer")],
            || Ok(()),
            || Ok(()),
            || Ok(()),
            || Err(String::from("interactive installer removed")),
        );

        assert_eq!(result, Err(String::from("interactive installer removed")));
    }
}
