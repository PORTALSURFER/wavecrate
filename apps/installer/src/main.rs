//! Windows installer entry point for Sempal.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
use std::env;

mod cleanup;
mod download;
mod install;
mod paths;
mod registry;
mod shortcuts;
mod ui;

const APP_NAME: &str = "SemPal";
#[cfg(target_os = "windows")]
const APP_PUBLISHER: &str = "SemPal";
#[cfg(target_os = "windows")]
const UNINSTALL_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SemPal";

fn main() -> Result<(), String> {
    run_with_args(
        env::args(),
        cleanup::run_uninstall,
        install::run_dry_run,
        ui::run_installer_app,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallerEntryCommand {
    Uninstall,
    DryRun,
    LaunchUi,
}

fn run_with_args<I, U, D, L>(
    args: I,
    run_uninstall: U,
    run_dry_run: D,
    run_ui: L,
) -> Result<(), String>
where
    I: IntoIterator<Item = String>,
    U: FnOnce() -> Result<(), String>,
    D: FnOnce() -> Result<(), String>,
    L: FnOnce() -> Result<(), String>,
{
    match select_entry_command(args) {
        InstallerEntryCommand::Uninstall => {
            if let Err(err) = run_uninstall() {
                eprintln!("Uninstall failed: {err}");
            }
            Ok(())
        }
        InstallerEntryCommand::DryRun => {
            if let Err(err) = run_dry_run() {
                eprintln!("Dry run failed: {err}");
            }
            Ok(())
        }
        InstallerEntryCommand::LaunchUi => run_ui(),
    }
}

fn select_entry_command<I>(args: I) -> InstallerEntryCommand
where
    I: IntoIterator<Item = String>,
{
    let mut command = InstallerEntryCommand::LaunchUi;
    for arg in args.into_iter().skip(1) {
        if arg == "--uninstall" {
            return InstallerEntryCommand::Uninstall;
        }
        if arg == "--dry-run" {
            command = InstallerEntryCommand::DryRun;
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::{InstallerEntryCommand, run_with_args, select_entry_command};
    use super::install::{PlanAction, plan_install};
    use std::cell::Cell;
    use std::fs;

    #[test]
    fn dry_run_plans_bundle_copies() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = temp.path().join("bundle");
        let install = temp.path().join("install");
        fs::create_dir_all(bundle.join("bin")).expect("bundle dir");
        fs::write(bundle.join("bin").join("sempal.exe"), "test").expect("exe");

        let plan = plan_install(&bundle, &install).expect("plan");
        let copies = plan
            .actions
            .iter()
            .filter(|action| matches!(action, PlanAction::Copy { .. }))
            .count();
        assert_eq!(copies, 1);
    }

    #[test]
    fn select_entry_command_defaults_to_launch_ui() {
        let command = select_entry_command(vec![String::from("sempal-installer")]);
        assert_eq!(command, InstallerEntryCommand::LaunchUi);
    }

    #[test]
    fn select_entry_command_uses_dry_run_when_requested() {
        let command = select_entry_command(vec![
            String::from("sempal-installer"),
            String::from("--dry-run"),
        ]);
        assert_eq!(command, InstallerEntryCommand::DryRun);
    }

    #[test]
    fn select_entry_command_prefers_uninstall_over_dry_run() {
        let command = select_entry_command(vec![
            String::from("sempal-installer"),
            String::from("--dry-run"),
            String::from("--uninstall"),
        ]);
        assert_eq!(command, InstallerEntryCommand::Uninstall);
    }

    #[test]
    fn run_with_args_dispatches_uninstall_without_launching_ui() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let ui_called = Cell::new(false);

        let result = run_with_args(
            vec![
                String::from("sempal-installer"),
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
                ui_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(uninstall_called.get());
        assert!(!dry_run_called.get());
        assert!(!ui_called.get());
    }

    #[test]
    fn run_with_args_dispatches_dry_run_without_launching_ui() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let ui_called = Cell::new(false);

        let result = run_with_args(
            vec![
                String::from("sempal-installer"),
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
                ui_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(!uninstall_called.get());
        assert!(dry_run_called.get());
        assert!(!ui_called.get());
    }

    #[test]
    fn run_with_args_launches_ui_by_default() {
        let uninstall_called = Cell::new(false);
        let dry_run_called = Cell::new(false);
        let ui_called = Cell::new(false);

        let result = run_with_args(
            vec![String::from("sempal-installer")],
            || {
                uninstall_called.set(true);
                Ok(())
            },
            || {
                dry_run_called.set(true);
                Ok(())
            },
            || {
                ui_called.set(true);
                Ok(())
            },
        );

        assert!(result.is_ok());
        assert!(!uninstall_called.get());
        assert!(!dry_run_called.get());
        assert!(ui_called.get());
    }

    #[test]
    fn run_with_args_propagates_ui_launch_errors() {
        let result = run_with_args(
            vec![String::from("sempal-installer")],
            || Ok(()),
            || Ok(()),
            || Err(String::from("ui failed")),
        );

        assert_eq!(result, Err(String::from("ui failed")));
    }
}
