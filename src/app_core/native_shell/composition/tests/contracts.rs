use super::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const OWNERSHIP_INVENTORY: &str =
    include_str!("../ownership_inventory.tsv");

#[test]
fn focused_rows_do_not_enable_idle_animation_when_transport_is_stopped() {
    let mut state = NativeShellState::new();
    let mut model = crate::app_core::native_shell::runtime_contract::AppModel::default();
    model.transport_running = false;
    model
        .browser
        .rows
        .push(crate::app_core::native_shell::runtime_contract::BrowserRowModel::new(
            0, "kick", 1, false, true,
        ));
    state.sync_from_model(&model);
    state.sync_from_model(&model);
    assert!(!state.needs_animation());

    let mut idle_model = crate::app_core::native_shell::runtime_contract::AppModel::default();
    idle_model.transport_running = false;
    let mut playing_model = crate::app_core::native_shell::runtime_contract::AppModel::default();
    playing_model.transport_running = true;
    state.sync_from_model(&playing_model);
    assert!(state.needs_animation());
    state.sync_from_model(&idle_model);
    assert!(!state.needs_animation());
}

#[test]
fn long_browser_labels_are_truncated_with_ellipsis() {
    let layout = ShellLayout::build(Vector2::new(620.0, 420.0));
    let mut state = NativeShellState::new();
    let mut model = crate::app_core::native_shell::runtime_contract::AppModel::default();
    model.browser.rows.push(crate::app_core::native_shell::runtime_contract::BrowserRowModel::new(
        0,
        "this_is_a_very_long_browser_row_label_that_should_truncate_in_native_shell_rendering_and_is_intentionally_longer_than_any_practical_row_width_even_on_narrow_compact_views.wav",
        1,
        false,
        false,
    ));
    state.sync_from_model(&model);
    let frame = state.build_frame(&layout, &model);
    let truncated = frame
        .text_runs
        .iter()
        .find(|run| run.text.starts_with("this_is_a"))
        .map(|run| run.text.as_str())
        .unwrap_or_default();
    assert!(truncated.ends_with("..."));
}

#[test]
fn canonical_frame_rebuild_is_deterministic_across_tiers() {
    let mut state = NativeShellState::new();
    let model = canonical_shell_model();
    state.sync_from_model(&model);
    for viewport in [Vector2::new(1280.0, 720.0), Vector2::new(2300.0, 1080.0)] {
        let layout = ShellLayout::build(viewport);
        let frame_a = state.build_frame(&layout, &model);
        let frame_b = state.build_frame(&layout, &model);
        assert_eq!(frame_a, frame_b);
        assert!(!frame_a.primitives.is_empty());
        assert!(!frame_a.text_runs.is_empty());
    }
}

#[test]
fn canonical_frame_contains_expected_sidebar_and_status_contract_text() {
    let mut state = NativeShellState::new();
    let model = canonical_shell_model();
    state.sync_from_model(&model);
    let layout = ShellLayout::build(Vector2::new(1280.0, 720.0));
    let frame = state.build_frame(&layout, &model);
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Active Pane:"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("rows: 48"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("col: 2/3"))
    );
    assert!(frame.text_runs.iter().any(|run| run.text == "kick"));
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("36 items"))
    );
    assert!(
        frame
            .text_runs
            .iter()
            .any(|run| run.text.contains("Loop engaged"))
    );
}

#[derive(Debug)]
struct OwnershipRule {
    pattern: String,
    owner: String,
    move_scope: String,
}

#[test]
fn native_shell_ownership_inventory_covers_every_module() {
    let rules = parse_ownership_rules();
    assert_owner_and_scope_sets_are_represented(&rules);
    let modules = native_shell_rs_modules();
    let mut matched_rules: BTreeMap<&str, usize> = rules
        .iter()
        .map(|rule| (rule.pattern.as_str(), 0))
        .collect();

    for module in &modules {
        let matches: Vec<_> = rules.iter().filter(|rule| rule.matches(module)).collect();
        assert_eq!(
            matches.len(),
            1,
            "{module} should match exactly one ownership rule, matched {:?}",
            matches
                .iter()
                .map(|rule| rule.pattern.as_str())
                .collect::<Vec<_>>()
        );
        *matched_rules.get_mut(matches[0].pattern.as_str()).unwrap() += 1;
    }

    let unused_rules: Vec<_> = matched_rules
        .iter()
        .filter_map(|(pattern, count)| (*count == 0).then_some(*pattern))
        .collect();
    assert!(
        unused_rules.is_empty(),
        "ownership inventory contains rules that match no modules: {unused_rules:?}"
    );
}

fn assert_owner_and_scope_sets_are_represented(rules: &[OwnershipRule]) {
    for expected_owner in [
        "radiant_generic",
        "compat_adapter",
        "sempal_composition",
        "sempal_fixture",
    ] {
        assert!(
            rules.iter().any(|rule| rule.owner == expected_owner),
            "ownership inventory should include at least one {expected_owner} rule"
        );
    }
    for expected_scope in [
        "stay_radiant",
        "compat_boundary",
        "OPT-187",
        "OPT-188",
        "OPT-189",
        "OPT-190",
        "OPT-191",
    ] {
        assert!(
            rules.iter().any(|rule| rule.move_scope == expected_scope),
            "ownership inventory should include at least one {expected_scope} rule"
        );
    }
}

fn parse_ownership_rules() -> Vec<OwnershipRule> {
    let mut rules = Vec::new();
    for (line_index, line) in OWNERSHIP_INVENTORY.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("pattern\t") {
            continue;
        }
        let columns: Vec<_> = line.split('\t').collect();
        assert_eq!(
            columns.len(),
            4,
            "ownership inventory line {} should have four tab-separated columns",
            line_index + 1
        );
        let owner = columns[1].to_owned();
        assert!(
            matches!(
                owner.as_str(),
                "radiant_generic" | "compat_adapter" | "sempal_composition" | "sempal_fixture"
            ),
            "unknown owner {owner:?} on ownership inventory line {}",
            line_index + 1
        );
        let move_scope = columns[2].to_owned();
        assert!(
            matches!(
                move_scope.as_str(),
                "stay_radiant"
                    | "compat_boundary"
                    | "OPT-187"
                    | "OPT-188"
                    | "OPT-189"
                    | "OPT-190"
                    | "OPT-191"
            ),
            "unknown move scope {move_scope:?} on ownership inventory line {}",
            line_index + 1
        );
        rules.push(OwnershipRule {
            pattern: columns[0].to_owned(),
            owner,
            move_scope,
        });
    }
    assert!(!rules.is_empty(), "ownership inventory should not be empty");
    rules
}

impl OwnershipRule {
    fn matches(&self, module: &str) -> bool {
        if let Some(prefix) = self.pattern.strip_suffix("/**") {
            module.starts_with(&format!("{prefix}/"))
        } else {
            self.pattern == module
        }
    }
}

fn native_shell_rs_modules() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/gui/native_shell");
    let mut modules = Vec::new();
    collect_rs_modules(&root, &root, &mut modules);
    let sempal_composition_root = root.join("../../../../../src/app_core/native_shell/composition");
    if sempal_composition_root.exists() {
        collect_rs_modules(
            &sempal_composition_root,
            &sempal_composition_root,
            &mut modules,
        );
    }
    modules.sort();
    modules.dedup();
    modules
}

fn collect_rs_modules(root: &Path, dir: &Path, modules: &mut Vec<String>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|error| {
        panic!(
            "failed to read native_shell directory {}: {error}",
            dir.display()
        )
    });
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to read native_shell entry: {error}"))
            .path();
        if path.is_dir() {
            collect_rs_modules(root, &path, modules);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            modules.push(relative_module_path(root, &path));
        }
    }
}

fn relative_module_path(root: &Path, path: &PathBuf) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|error| {
            panic!(
                "native shell module {} should be under {}: {error}",
                path.display(),
                root.display()
            )
        })
        .to_string_lossy()
        .replace('\\', "/")
}

