//! Developer utility to rebuild the ANN index from embeddings.

use rusqlite::Connection;
use sempal_analysis_admin::cli_support;
use std::path::PathBuf;

fn main() {
    cli_support::run_command(run);
}

#[derive(Debug)]
struct Options {
    db_path: PathBuf,
}

fn run() -> Result<(), String> {
    let Some(options) = parse_args(std::env::args().skip(1).collect())? else {
        return Ok(());
    };
    let conn =
        Connection::open(&options.db_path).map_err(|err| format!("Open DB failed: {err}"))?;
    sempal::analysis::rebuild_ann_index(&conn)?;
    println!("Rebuilt ANN index for {}", options.db_path.display());
    Ok(())
}

fn parse_args(args: Vec<String>) -> Result<Option<Options>, String> {
    if cli_support::help_requested(&args) {
        print_help();
        return Ok(None);
    }
    let mut db_path = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--db" => {
                let value = it
                    .next()
                    .ok_or_else(|| "Missing value for --db".to_string())?;
                db_path = Some(PathBuf::from(value));
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }
    let db_path = match db_path {
        Some(path) => path,
        None => cli_support::resolve_library_db_path(None)?,
    };
    Ok(Some(Options { db_path }))
}

fn print_help() {
    println!("Usage: sempal-ann-rebuild [--db <path>]");
    println!();
    println!("Options:");
    println!("  --db <path>  Path to library.db (defaults to app data dir)");
}

#[cfg(test)]
mod tests {
    use super::parse_args;
    use std::path::PathBuf;

    #[test]
    fn parse_args_defaults_db_path_when_omitted() {
        let options = parse_args(Vec::new())
            .expect("parse succeeds")
            .expect("help not requested");
        assert!(options.db_path.ends_with(PathBuf::from("library.db")));
    }

    #[test]
    fn parse_args_rejects_unknown_argument() {
        let err = parse_args(vec!["--wat".to_string()]).expect_err("unknown arg should fail");
        assert!(err.contains("Unknown argument: --wat"));
    }
}
