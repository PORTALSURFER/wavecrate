//! Source-quality guardrails for the Wavecrate/Radiant GUI boundary.

use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn gui_module_stays_a_pure_radiant_reexport_boundary() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source = fs::read_to_string(format!("{manifest_dir}/src/gui/mod.rs"))
        .expect("src/gui/mod.rs should be readable");

    for forbidden in ["pub trait ", "impl ", "fn ", "struct ", "enum ", "const "] {
        assert!(
            !source.contains(forbidden),
            "src/gui should stay a pure Radiant re-export boundary; found `{forbidden}`"
        );
    }
}

#[test]
fn architecture_docs_call_out_large_gui_import_lists() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source = fs::read_to_string(format!("{manifest_dir}/docs/ARCHITECTURE.md"))
        .expect("docs/ARCHITECTURE.md should be readable");

    for required in [
        "large import lists are architecture signals",
        "split the module by responsibility",
        "move reusable",
        "GUI behavior into Radiant",
        "avoid wildcard imports",
    ] {
        assert!(
            source.contains(required),
            "docs/ARCHITECTURE.md should preserve the GUI import hygiene rule: missing `{required}`"
        );
    }
}

#[test]
fn production_gui_modules_do_not_use_top_level_wildcard_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gui_root = manifest_dir.join("src/gui_app");
    let mut offenders = Vec::new();
    collect_top_level_wildcard_imports(&gui_root, &mut offenders);

    assert!(
        offenders.is_empty(),
        "production GUI modules should use explicit imports instead of top-level wildcard imports:\n{}",
        offenders.join("\n")
    );
}

fn collect_top_level_wildcard_imports(dir: &Path, offenders: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("{dir:?} should be readable: {err}"))
    {
        let entry = entry.expect("GUI source directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_top_level_wildcard_imports(&path, offenders);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs")
            || is_test_source(&path)
        {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{} should be readable: {err}", path.display()));
        for (line_index, line) in source.lines().enumerate() {
            if line.starts_with("use super::*") {
                let relative = path
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(&path);
                offenders.push(format!("{}:{}", relative.display(), line_index + 1));
            }
        }
    }
}

fn is_test_source(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("tests.rs")
        || path
            .components()
            .any(|component| component.as_os_str() == "tests")
        || path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with("_tests.rs"))
}
