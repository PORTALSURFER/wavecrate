//! Source-quality guardrails for the Wavecrate/Radiant GUI boundary.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use radiant::guardrails::NonBlockingGuardrail;

struct FacadeBudget {
    path: &'static str,
    max_lines: usize,
    max_exports: usize,
    owner: &'static str,
}

const WAVECRATE_FACADE_BUDGETS: &[FacadeBudget] = &[
    FacadeBudget {
        path: "src/native_app/sample_library/folder_browser.rs",
        max_lines: 180,
        max_exports: 32,
        owner: "OPT-541: folder-browser root stays a thin module/export facade",
    },
    FacadeBudget {
        path: "src/app_core/app_api.rs",
        max_lines: 150,
        max_exports: 28,
        owner: "OPT-541: app-api remains the owned legacy-crossing allowlist",
    },
    FacadeBudget {
        path: "src/app_core/actions/mod.rs",
        max_lines: 240,
        max_exports: 66,
        owner: "OPT-541: app-core action facade exports remain deliberate",
    },
    FacadeBudget {
        path: "src/native_app/test_support.rs",
        max_lines: 40,
        max_exports: 12,
        owner: "OPT-541: test support root stays a test-only re-export facade",
    },
];

#[test]
fn wavecrate_does_not_reintroduce_local_radiant_gui_prelude() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let prelude_path = Path::new(manifest_dir).join("src/ui_primitives/mod.rs");
    assert!(
        !prelude_path.exists(),
        "Wavecrate should not maintain a local Radiant GUI prelude; import radiant::prelude or explicit radiant::gui subsystems instead"
    );

    let source_root = Path::new(manifest_dir).join("src");
    let mut offenders = Vec::new();
    collect_matching_source_lines(
        &source_root,
        Path::new(manifest_dir),
        &mut offenders,
        |line| line.contains("crate::ui_primitives") || line.contains("ui_primitives::"),
    );
    assert!(
        offenders.is_empty(),
        "Wavecrate code should import Radiant directly instead of using a local ui_primitives facade:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn target_docs_call_out_large_gui_import_lists() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source = fs::read_to_string(format!("{manifest_dir}/docs/TARGET.md"))
        .expect("docs/TARGET.md should be readable");

    for required in [
        "large import lists are architecture signals",
        "split the module by responsibility",
        "move reusable",
        "GUI behavior into Radiant",
        "avoid wildcard imports",
    ] {
        assert!(
            source.contains(required),
            "docs/TARGET.md should preserve the GUI import hygiene rule: missing `{required}`"
        );
    }
}

#[test]
fn agent_instructions_call_out_large_gui_import_lists() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source = fs::read_to_string(format!("{manifest_dir}/AGENTS.md"))
        .expect("AGENTS.md should be readable");

    for required in [
        "large import lists are architecture signals",
        "split the module by responsibility",
        "move reusable",
        "GUI behavior into Radiant",
        "facade may wire",
    ] {
        assert!(
            source.contains(required),
            "AGENTS.md should preserve the GUI import hygiene rule: missing `{required}`"
        );
    }
}

#[test]
fn wavecrate_root_facades_stay_within_owned_size_budgets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for budget in WAVECRATE_FACADE_BUDGETS {
        let path = manifest_dir.join(budget.path);
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("{} should be readable: {err}", path.display()));
        let line_count = source.lines().count();
        let export_count = source.lines().filter(|line| is_export_line(line)).count();

        assert!(
            line_count <= budget.max_lines,
            "{} has {line_count} lines; budget is {}. {}",
            budget.path,
            budget.max_lines,
            budget.owner
        );
        assert!(
            export_count <= budget.max_exports,
            "{} has {export_count} export lines; budget is {}. {}",
            budget.path,
            budget.max_exports,
            budget.owner
        );
    }
}

#[test]
fn production_app_core_legacy_crossings_go_through_app_api() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let app_core_root = manifest_dir.join("src/app_core");
    let mut offenders = Vec::new();
    collect_app_core_legacy_crossings(&app_core_root, &manifest_dir, &mut offenders);

    assert!(
        offenders.is_empty(),
        "production app-core code must import legacy app modules through src/app_core/app_api.rs:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn app_api_state_dto_inventory_points_at_active_followup() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let source = fs::read_to_string(format!("{manifest_dir}/src/app_core/app_api.rs"))
        .expect("src/app_core/app_api.rs should be readable");

    assert!(
        !source.contains("OPT-676"),
        "app_api state DTO cleanup must not point at OPT-676; that issue now tracks completed browser tag-sidebar projection work"
    );
    assert!(
        source.contains("Browser/source/map/audio state DTOs") && source.contains("| `OPT-677` |"),
        "app_api migration inventory should map remaining legacy state DTO exports to OPT-677"
    );
    assert!(
        source.contains("while OPT-677 replaces legacy state usage"),
        "test-only legacy state DTO exports should mention the active OPT-677 follow-up"
    );
}

#[test]
fn production_native_app_modules_do_not_import_test_support_facade() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let native_app_root = manifest_dir.join("src/native_app");
    let mut offenders = Vec::new();
    collect_native_app_test_support_imports(&native_app_root, &manifest_dir, &mut offenders);

    assert!(
        offenders.is_empty(),
        "production native-app modules must not import the cfg(test) test_support facade:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn production_gui_modules_do_not_use_top_level_wildcard_imports() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gui_root = manifest_dir.join("src/native_app/app_chrome");
    let mut offenders = Vec::new();
    collect_top_level_wildcard_imports(&gui_root, &mut offenders);

    assert!(
        offenders.is_empty(),
        "production GUI modules should use explicit imports instead of top-level wildcard imports:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn cross_crate_public_wildcard_reexports_are_explicitly_audited() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src");
    let mut actual = BTreeSet::new();
    collect_cross_crate_public_wildcard_reexports(&source_root, &manifest_dir, &mut actual);

    let expected: BTreeSet<String> = BTreeSet::new();

    assert_eq!(
        actual, expected,
        "cross-crate public wildcard re-exports must be audited compatibility shims; \
         narrow the export or add an explicit ownership note before updating this list"
    );
}

#[test]
fn native_app_ui_update_paths_do_not_call_blocking_business_apis() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let native_app_root = manifest_dir.join("src/native_app");

    wavecrate_non_blocking_guardrail()
        .scan_roots([native_app_root])
        .expect("native-app UI/update paths must offload filesystem, database, thread, sleep, clipboard, and other blocking business work through Radiant BusinessRuntime or a platform service");
}

fn collect_app_core_legacy_crossings(dir: &Path, manifest_dir: &Path, offenders: &mut Vec<String>) {
    for_rust_source_file(dir, &mut |path| {
        if is_test_source(path) || path.ends_with("app_api.rs") {
            return;
        }
        collect_matching_lines(path, manifest_dir, offenders, app_core_legacy_crossing);
    });
}

fn collect_native_app_test_support_imports(
    dir: &Path,
    manifest_dir: &Path,
    offenders: &mut Vec<String>,
) {
    for_rust_source_file(dir, &mut |path| {
        if is_test_source(path) || path.ends_with("test_support.rs") {
            return;
        }
        collect_matching_lines(
            path,
            manifest_dir,
            offenders,
            native_app_test_support_import,
        );
    });
}

fn collect_top_level_wildcard_imports(dir: &Path, offenders: &mut Vec<String>) {
    for_rust_source_file(dir, &mut |path| {
        if is_test_source(path) {
            return;
        }
        let source = read_source(path);
        for (line_index, line) in source.lines().enumerate() {
            if line.starts_with("use super::*") {
                let relative = path
                    .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(path);
                offenders.push(format!("{}:{}", relative.display(), line_index + 1));
            }
        }
    });
}

fn collect_cross_crate_public_wildcard_reexports(
    dir: &Path,
    manifest_dir: &Path,
    actual: &mut BTreeSet<String>,
) {
    for_rust_source_file(dir, &mut |path| {
        let source = read_source(path);
        for line in source
            .lines()
            .filter_map(cross_crate_public_wildcard_target)
        {
            let relative = path.strip_prefix(manifest_dir).unwrap_or(path);
            actual.insert(format!("{}:{line}", relative.display()).replace('\\', "/"));
        }
    });
}

fn for_rust_source_file(dir: &Path, visit: &mut impl FnMut(&Path)) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("{dir:?} should be readable: {err}"))
    {
        let entry = entry.expect("source directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            for_rust_source_file(&path, visit);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        visit(&path);
    }
}

fn collect_matching_lines(
    path: &Path,
    manifest_dir: &Path,
    offenders: &mut Vec<String>,
    matches_line: impl Fn(&str) -> bool,
) {
    let source = read_source(path);
    let mut previous_line_was_cfg_test = false;
    for (line_index, line) in source.lines().enumerate() {
        let cfg_test_line = is_cfg_test_line(line);
        if matches_line(line) && !previous_line_was_cfg_test {
            let relative = path.strip_prefix(manifest_dir).unwrap_or(path);
            offenders.push(format!(
                "{}:{}: {}",
                relative.display(),
                line_index + 1,
                line.trim()
            ));
        }
        previous_line_was_cfg_test = cfg_test_line;
    }
}

fn collect_matching_source_lines(
    dir: &Path,
    manifest_dir: &Path,
    offenders: &mut Vec<String>,
    matches_line: impl Copy + Fn(&str) -> bool,
) {
    for_rust_source_file(dir, &mut |path| {
        collect_matching_lines(path, manifest_dir, offenders, matches_line);
    });
}

fn read_source(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("{} should be readable: {err}", path.display()))
}

fn app_core_legacy_crossing(line: &str) -> bool {
    let code = line.trim();
    !is_comment_or_empty(code)
        && (code.contains("crate::app::controller")
            || code.contains("crate::app::state")
            || code.contains("crate::app::view_model"))
}

fn native_app_test_support_import(line: &str) -> bool {
    let code = line.trim();
    !is_comment_or_empty(code)
        && (code.contains("native_app::test_support")
            || code.contains("super::test_support")
            || code.contains("test_support::"))
}

fn is_export_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("pub use ")
        || trimmed.starts_with("pub(crate) use ")
        || trimmed.starts_with("pub(in ")
        || trimmed.starts_with("pub const ")
        || trimmed.starts_with("pub(crate) const ")
        || trimmed.starts_with("pub type ")
        || trimmed.starts_with("pub(crate) type ")
}

fn wavecrate_non_blocking_guardrail() -> NonBlockingGuardrail {
    let mut guardrail = NonBlockingGuardrail::app_update_paths()
        .forbid_token(
            "open_for_user_metadata_write",
            "metadata database write",
            "schedule metadata persistence through context.business()",
        )
        .forbid_token(
            "external_clipboard::",
            "direct clipboard access",
            "request clipboard work through a typed Radiant platform service",
        )
        .forbid_token(
            ".canonicalize(",
            "filesystem path resolution",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            ".is_file(",
            "filesystem metadata check",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            ".is_dir(",
            "filesystem metadata check",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            ".exists(",
            "filesystem metadata check",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::metadata(",
            "filesystem metadata check",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::read(",
            "filesystem read",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::read_dir(",
            "filesystem directory scan",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::read_to_string(",
            "filesystem read",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::write(",
            "filesystem write",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::create_dir",
            "filesystem write",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::remove_file(",
            "filesystem write",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::remove_dir",
            "filesystem write",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::rename(",
            "filesystem write",
            "schedule filesystem work through context.business()",
        )
        .forbid_token(
            "fs::copy(",
            "filesystem write",
            "schedule filesystem work through context.business()",
        );

    for (fragment, reason) in [
        ("/tests/", "native app behavior tests"),
        ("tests.rs", "native app behavior tests"),
        ("src/native_app/tests.rs", "native app behavior tests"),
        ("src/native_app/tests", "native app behavior tests"),
        (
            "src/native_app/app/state/source_scan_worker.rs",
            "source scan worker boundary",
        ),
        (
            "src/native_app/audio/sample_load_actions/cache/workers.rs",
            "waveform cache business worker",
        ),
        (
            "src/native_app/audio/sample_load_actions/deferred_drop.rs",
            "deferred file-drop worker boundary",
        ),
        (
            "src/native_app/file_actions/wav_normalize.rs",
            "audio file worker",
        ),
        (
            "src/native_app/metadata/persistence.rs",
            "metadata persistence worker",
        ),
        (
            "src/native_app/sample_library/file_actions/wav_normalize.rs",
            "audio file worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/file_move_execution.rs",
            "file operation worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/file_move_transaction.rs",
            "file operation transaction worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/collections/assignment.rs",
            "collection persistence worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/delete_workflow.rs",
            "delete workflow worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/filesystem_refresh.rs",
            "filesystem refresh worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/rename_execution.rs",
            "rename worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/scanning.rs",
            "source scanning worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/scanning/file_entry_metadata.rs",
            "source scanning metadata worker",
        ),
        (
            "src/native_app/sample_library/folder_browser/source_scan_cache.rs",
            "source scan cache worker",
        ),
        (
            "src/native_app/sample_library/drag_drop_actions/external.rs",
            "typed external drag platform handoff",
        ),
        (
            "src/native_app/sample_library/native_file_drop_actions.rs",
            "typed native file-drop platform handoff",
        ),
        (
            "src/native_app/sample_library/sample_collections/persistence.rs",
            "collection persistence worker",
        ),
        (
            "src/native_app/sample_library/sample_ratings.rs",
            "rating persistence scheduling boundary",
        ),
        (
            "src/native_app/sample_library/source_watcher/classification.rs",
            "source watcher worker",
        ),
        (
            "src/native_app/sample_library/source_watcher/handle.rs",
            "source watcher worker",
        ),
        (
            "src/native_app/sample_library/source_watcher/roots.rs",
            "source watcher worker",
        ),
        (
            "src/native_app/sample_library/trash_actions/movement.rs",
            "trash movement worker",
        ),
        (
            "src/native_app/waveform/audio_file/",
            "waveform cache and decode workers",
        ),
    ] {
        guardrail = guardrail.allow_path_fragment(fragment, reason);
    }

    guardrail
}

fn is_comment_or_empty(line: &str) -> bool {
    line.is_empty() || line.starts_with("//")
}

fn is_cfg_test_line(line: &str) -> bool {
    matches!(line.trim(), "#[cfg(test)]" | "#[cfg(any(test, doctest))]")
}

fn cross_crate_public_wildcard_target(line: &str) -> Option<String> {
    let target = line
        .trim()
        .strip_prefix("pub use ")?
        .strip_suffix(';')?
        .trim();
    if !target.ends_with("::*") || !target.contains("::") {
        return None;
    }
    let root = target.split("::").next().unwrap_or_default();
    matches!(
        root,
        "radiant" | "reson" | "wavecrate_analysis" | "wavecrate_library" | "wavecrate_scan"
    )
    .then(|| target.to_owned())
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
