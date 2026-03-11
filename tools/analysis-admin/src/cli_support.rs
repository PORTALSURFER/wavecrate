//! Shared command-line helpers for the analysis-admin developer tools.

use std::path::{Path, PathBuf};

/// Run one analysis-admin command and translate failures into a non-zero exit.
///
/// This keeps the tiny binaries consistent: each command prints the user-facing
/// error string to stderr and exits with status `1` when the command fails.
pub fn run_command(run: impl FnOnce() -> Result<(), String>) {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

/// Resolve the default `library.db` path unless an explicit path was provided.
///
/// Commands that operate on the app library share the same app-data default.
pub fn resolve_library_db_path(db_path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = db_path {
        return Ok(path.to_path_buf());
    }
    let root = sempal::app_dirs::app_root_dir().map_err(|err| err.to_string())?;
    Ok(root.join(sempal::sample_sources::library::LIBRARY_DB_FILE_NAME))
}

/// Return whether any command-line argument requested help output.
pub fn help_requested(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--help" || arg == "-h")
}
