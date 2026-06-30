//! Release contract checks for active targets and nightly asset labels.

use std::collections::{BTreeMap, BTreeSet};

use toml::Value;

const RELEASE_CONTRACT: &str = include_str!("../release_contract.toml");
const RELEASE_WORKFLOW: &str = include_str!("../.github/workflows/release-build.yml");

#[test]
fn platform_labels_are_active_release_labels_only() {
    let contract = parse_contract();
    let targets = contract_targets(&contract);
    let platform_labels = platform_labels(&contract);
    let active_labels = active_labels_for_targets(&targets);

    assert_eq!(
        platform_labels, active_labels,
        "release_contract.toml [platform_labels] must describe only active target labels"
    );
    assert!(
        !platform_labels.contains("linux"),
        "Linux is not an active release target and must not appear in [platform_labels]"
    );
}

#[test]
fn release_contract_targets_match_nightly_workflow_matrix() {
    let contract = parse_contract();
    let targets = contract_targets(&contract);
    let workflow_targets = workflow_release_targets();

    assert_eq!(
        workflow_targets,
        active_target_matrix(&targets),
        "release_contract.toml targets must match the nightly release workflow matrix"
    );
}

#[test]
fn release_contract_template_emits_supported_nightly_asset_names() {
    let contract = parse_contract();
    let targets = contract_targets(&contract);
    let template = contract
        .get("templates")
        .and_then(Value::as_table)
        .and_then(|templates| templates.get("nightly_asset"))
        .and_then(Value::as_str)
        .expect("nightly_asset template");

    let asset_names: BTreeSet<String> = active_target_matrix(&targets)
        .into_iter()
        .map(|target| {
            template
                .replace("{APP_NAME}", app_name(&contract))
                .replace("{platform}", &target.platform)
                .replace("{arch}", &target.arch)
        })
        .collect();

    assert_eq!(
        asset_names,
        BTreeSet::from([
            "wavecrate-nightly-macos-aarch64.zip".to_string(),
            "wavecrate-nightly-macos-x86_64.zip".to_string(),
            "wavecrate-nightly-windows-x86_64.zip".to_string(),
        ])
    );
    assert!(
        asset_names.iter().all(|name| !name.contains("linux")),
        "nightly assets generated from the release contract must not include Linux"
    );
}

fn parse_contract() -> Value {
    RELEASE_CONTRACT
        .parse::<Value>()
        .expect("release_contract.toml parses")
}

fn app_name(contract: &Value) -> &str {
    contract
        .get("app_name")
        .and_then(Value::as_str)
        .expect("app_name")
}

fn contract_targets(contract: &Value) -> Vec<String> {
    contract
        .get("targets")
        .and_then(Value::as_array)
        .expect("targets")
        .iter()
        .map(|target| target.as_str().expect("target triple").to_string())
        .collect()
}

fn platform_labels(contract: &Value) -> BTreeSet<String> {
    contract
        .get("platform_labels")
        .and_then(Value::as_table)
        .expect("platform_labels")
        .values()
        .map(|label| label.as_str().expect("label string").to_string())
        .collect()
}

fn active_labels_for_targets(targets: &[String]) -> BTreeSet<String> {
    active_target_matrix(targets)
        .into_iter()
        .flat_map(|target| [target.platform, target.arch])
        .collect()
}

fn active_target_matrix(targets: &[String]) -> BTreeSet<ReleaseTarget> {
    targets
        .iter()
        .map(|target| ReleaseTarget {
            target: target.clone(),
            platform: platform_for_target(target),
            arch: arch_for_target(target),
        })
        .collect()
}

fn platform_for_target(target: &str) -> String {
    if target.contains("-pc-windows-") {
        return "windows".to_string();
    }
    if target.ends_with("-apple-darwin") {
        return "macos".to_string();
    }
    if target.contains("-unknown-linux-") {
        return "linux".to_string();
    }
    panic!("unsupported release target triple: {target}");
}

fn arch_for_target(target: &str) -> String {
    target
        .split_once('-')
        .map(|(arch, _)| arch.to_string())
        .expect("target arch")
}

fn workflow_release_targets() -> BTreeSet<ReleaseTarget> {
    let mut targets = BTreeSet::new();
    let mut entry = BTreeMap::new();
    let mut in_matrix_include = false;

    for line in RELEASE_WORKFLOW.lines() {
        let trimmed = line.trim();
        if trimmed == "include:" {
            in_matrix_include = true;
            continue;
        }
        if !in_matrix_include {
            continue;
        }
        if trimmed == "steps:" {
            break;
        }
        if trimmed.starts_with("- os:") {
            insert_workflow_entry(&mut targets, &mut entry);
            continue;
        }
        if let Some((key, value)) = trimmed.split_once(':') {
            let value = value.trim();
            if matches!(key, "target" | "platform" | "arch") && !value.is_empty() {
                entry.insert(key.to_string(), value.to_string());
            }
        }
    }
    insert_workflow_entry(&mut targets, &mut entry);

    targets
}

fn insert_workflow_entry(
    targets: &mut BTreeSet<ReleaseTarget>,
    entry: &mut BTreeMap<String, String>,
) {
    let Some(target) = entry.remove("target") else {
        entry.clear();
        return;
    };
    let platform = entry.remove("platform").expect("workflow platform");
    let arch = entry.remove("arch").expect("workflow arch");
    targets.insert(ReleaseTarget {
        target,
        platform,
        arch,
    });
    entry.clear();
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReleaseTarget {
    target: String,
    platform: String,
    arch: String,
}
