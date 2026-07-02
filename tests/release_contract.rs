//! Release contract checks for active targets and release asset labels.

use std::collections::{BTreeMap, BTreeSet};

use toml::Value;

const RELEASE_CONTRACT: &str = include_str!("../release_contract.toml");
const NIGHTLY_WORKFLOW: &str = include_str!("../.github/workflows/release-build.yml");
const RELEASE_TRAIN_PREP_WORKFLOW: &str =
    include_str!("../.github/workflows/release-train-prepare.yml");
const RC_WORKFLOW: &str = include_str!("../.github/workflows/release-rc.yml");
const STABLE_WORKFLOW: &str = include_str!("../.github/workflows/release-stable.yml");
const RELEASE_TRAIN_PREP_SCRIPT: &str =
    include_str!("../scripts/internal/release/prepare_release_train.py");
const ASSEMBLE_RELEASE_FILES_SCRIPT: &str =
    include_str!("../scripts/internal/release/assemble_release_files.sh");
const BUILD_RELEASE_ARTIFACT_SCRIPT: &str =
    include_str!("../scripts/internal/release/build_release_artifact.sh");
const CHECKOUT_RADIANT_SCRIPT: &str =
    include_str!("../scripts/internal/release/checkout_radiant_submodule.sh");
const RELEASE_ZIP_SCRIPT: &str = include_str!("../scripts/internal/release/build_release_zip.sh");
const RELEASE_LOG_SCRIPT: &str =
    include_str!("../scripts/internal/release/generate_release_log.sh");
const SIGN_RELEASE_CHECKSUMS_SCRIPT: &str =
    include_str!("../scripts/internal/release/sign_release_checksums.sh");
const VALIDATE_PROMOTED_RC_SCRIPT: &str =
    include_str!("../scripts/internal/release/validate_promoted_rc_release.py");
const UPDATER_ASSET_NAMES: &str = include_str!("../src/updater/asset_names.rs");

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
    let workflow_targets = workflow_release_targets(NIGHTLY_WORKFLOW);

    assert_eq!(
        workflow_targets,
        active_target_matrix(&targets),
        "release_contract.toml targets must match the nightly release workflow matrix"
    );
}

#[test]
fn release_contract_targets_match_manual_release_workflow_matrices() {
    let contract = parse_contract();
    let targets = contract_targets(&contract);
    let active = active_target_matrix(&targets);

    assert_eq!(
        workflow_release_targets(RC_WORKFLOW),
        active,
        "RC workflow matrix must match release_contract.toml targets"
    );
    assert_eq!(
        workflow_release_targets(STABLE_WORKFLOW),
        active,
        "stable workflow matrix must match release_contract.toml targets"
    );
}

#[test]
fn release_contract_declares_required_channels() {
    let contract = parse_contract();
    let channels: BTreeSet<_> = contract
        .get("channels")
        .and_then(Value::as_array)
        .expect("channels")
        .iter()
        .map(|channel| channel.as_str().expect("channel string"))
        .collect();

    assert_eq!(channels, BTreeSet::from(["nightly", "rc", "stable"]));
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

#[test]
fn release_packager_uses_contract_nightly_asset_name_without_build_number() {
    let contract = parse_contract();
    let nightly_template = contract
        .get("templates")
        .and_then(Value::as_table)
        .and_then(|templates| templates.get("nightly_asset"))
        .and_then(Value::as_str)
        .expect("nightly_asset template");
    let expected_assignment = format!(
        r#"ZIP_NAME="{}""#,
        nightly_template
            .replace("{APP_NAME}", "${APP_NAME}")
            .replace("{platform}", "${PLATFORM}")
            .replace("{arch}", "${ARCH}")
    );
    let expected_updater_format = nightly_template
        .replace("{APP_NAME}", "{APP_NAME}")
        .replace("{platform}", "{platform}")
        .replace("{arch}", "{arch}");

    assert!(
        RELEASE_ZIP_SCRIPT.contains(&expected_assignment),
        "nightly packager ZIP_NAME must match release_contract.toml: {expected_assignment}"
    );
    assert!(
        UPDATER_ASSET_NAMES.contains(&expected_updater_format),
        "GitHub updater nightly asset lookup must match release_contract.toml: {expected_updater_format}"
    );
    assert!(
        !RELEASE_ZIP_SCRIPT.contains("nightly${BUILD_LABEL}"),
        "rolling GitHub nightly assets must not include build-number labels"
    );
    assert!(
        RELEASE_ZIP_SCRIPT.contains(r#"printf "%s  %s\n" "$SHA" "$ZIP_NAME""#),
        "checksums-entry.txt must list the exact zip filename selected by the packager"
    );
}

#[test]
fn nightly_workflow_publishes_packager_outputs_as_github_assets() {
    assert!(
        NIGHTLY_WORKFLOW.contains("scripts/internal/release/build_release_artifact.sh \\"),
        "nightly workflow must use the shared release artifact builder"
    );
    assert!(
        NIGHTLY_WORKFLOW.contains("--channel nightly \\"),
        "nightly workflow must call the packager in nightly mode"
    );
    assert!(
        NIGHTLY_WORKFLOW.contains("scripts/internal/release/assemble_release_files.sh"),
        "GitHub nightly release must publish the zip filenames emitted by the packager"
    );
    assert!(
        NIGHTLY_WORKFLOW.contains("--checksum-name checksums-nightly.txt"),
        "GitHub nightly checksums must be assembled from packager checksum entries"
    );
    assert!(
        NIGHTLY_WORKFLOW.contains("files: dist/release/*"),
        "GitHub nightly release must upload the assembled dist/release assets"
    );
    assert!(
        !NIGHTLY_WORKFLOW.contains("wavecrate-nightly-b"),
        "rolling GitHub nightly workflow must not introduce build-numbered asset names"
    );
}

#[test]
fn release_contract_templates_emit_supported_rc_and_stable_asset_names() {
    let contract = parse_contract();
    let targets = contract_targets(&contract);
    let templates = contract
        .get("templates")
        .and_then(Value::as_table)
        .expect("templates");
    let rc_template = templates
        .get("rc_asset")
        .and_then(Value::as_str)
        .expect("rc_asset template");
    let stable_template = templates
        .get("stable_asset")
        .and_then(Value::as_str)
        .expect("stable_asset template");

    let rc_names: BTreeSet<String> = active_target_matrix(&targets)
        .iter()
        .map(|target| apply_asset_template(rc_template, app_name(&contract), target, "19.1.0", "2"))
        .collect();
    let stable_names: BTreeSet<String> = active_target_matrix(&targets)
        .iter()
        .map(|target| {
            apply_asset_template(stable_template, app_name(&contract), target, "19.1.0", "2")
        })
        .collect();

    assert_eq!(
        rc_names,
        BTreeSet::from([
            "wavecrate-19.1.0-rc.2-macos-aarch64.zip".to_string(),
            "wavecrate-19.1.0-rc.2-macos-x86_64.zip".to_string(),
            "wavecrate-19.1.0-rc.2-windows-x86_64.zip".to_string(),
        ])
    );
    assert_eq!(
        stable_names,
        BTreeSet::from([
            "wavecrate-19.1.0-macos-aarch64.zip".to_string(),
            "wavecrate-19.1.0-macos-x86_64.zip".to_string(),
            "wavecrate-19.1.0-windows-x86_64.zip".to_string(),
        ])
    );
}

#[test]
fn stable_workflow_requires_matching_rc_before_publish() {
    assert!(
        STABLE_WORKFLOW.contains("requires at least one RC tag"),
        "stable workflow must fail when no RC exists"
    );
    assert!(
        STABLE_WORKFLOW.contains("Latest RC ${rc_tag} points at"),
        "stable workflow must require the latest RC tag to point at the stable target commit"
    );
}

#[test]
fn stable_workflow_validates_promoted_rc_release_before_publish() {
    assert!(
        STABLE_WORKFLOW.contains("scripts/internal/release/validate_promoted_rc_release.py \\"),
        "stable workflow must validate the promoted RC GitHub release"
    );
    assert!(
        STABLE_WORKFLOW.contains("--rc-tag \"${{ steps.resolve.outputs.rc_tag }}\""),
        "stable workflow must validate the exact latest RC tag selected by resolve"
    );
    assert!(
        STABLE_WORKFLOW.contains("--repo \"${{ github.repository }}\""),
        "stable workflow must validate the RC release in the current GitHub repository"
    );
    assert!(
        STABLE_WORKFLOW.contains("--checksum-public-key"),
        "stable workflow must verify the promoted RC checksum signature"
    );
    let validate_position = STABLE_WORKFLOW
        .find("Validate promoted RC GitHub release artifacts")
        .expect("stable workflow validation step");
    let test_job_position = STABLE_WORKFLOW
        .find("name: Run stable validation tests")
        .expect("stable workflow test job");
    assert!(
        validate_position < test_job_position,
        "stable workflow must validate the promoted RC before stable test/build jobs start"
    );
}

#[test]
fn rc_and_stable_workflows_use_structured_release_log_generator() {
    for (name, workflow) in [
        ("RC workflow", RC_WORKFLOW),
        ("stable workflow", STABLE_WORKFLOW),
    ] {
        assert!(
            workflow.contains("fetch-depth: 0"),
            "{name} must fetch full history so release log ranges can be generated"
        );
        assert!(
            workflow.contains("tool: git-cliff"),
            "{name} must install git-cliff for generated release logs"
        );
        assert!(
            workflow.contains("scripts/internal/release/generate_release_log.sh \\"),
            "{name} must invoke the shared structured release-log generator"
        );
        assert!(
            workflow.contains("body_path: dist/release/release-log.md"),
            "{name} must publish the generated release log as the GitHub release body"
        );
    }
    assert!(
        !RC_WORKFLOW.contains("Release candidate for final validation."),
        "RC workflow must not fall back to a manual-only minimal release body"
    );
    assert!(
        !STABLE_WORKFLOW.contains("Stable release promoted from ${RC_TAG}."),
        "stable workflow must not fall back to a manual-only minimal release body"
    );
}

#[test]
fn structured_release_log_generator_declares_required_sections() {
    for section in [
        "## Release Metadata",
        "## Artifacts",
        "## Checksums",
        "## Manual Notes",
        "## Generated Changes",
    ] {
        assert!(
            RELEASE_LOG_SCRIPT.contains(section),
            "release log generator must declare {section}"
        );
    }
    assert!(
        RELEASE_LOG_SCRIPT.contains("git cliff"),
        "release log generator must try git-cliff before using the fallback"
    );
    assert!(
        RELEASE_LOG_SCRIPT.contains("git log --no-merges --pretty=format:'- %s'"),
        "release log generator must keep a deterministic commit-list fallback"
    );
}

#[test]
fn release_train_prep_workflow_is_manual_and_explicit_about_branch_pushes() {
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW.contains("workflow_dispatch:"),
        "release train prep must be a manual workflow"
    );
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW.contains("push_branch:"),
        "release train prep must require an explicit push_branch input"
    );
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW.contains("default: false"),
        "release train prep must default to dry-run/no-push"
    );
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW.contains("scripts/internal/release/prepare_release_train.py"),
        "release train prep workflow must invoke the shared prep script"
    );
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW.contains("--dry-run"),
        "release train prep workflow must support validation without mutation"
    );
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW.contains("--push"),
        "release train prep workflow must only push behind an explicit input"
    );
}

#[test]
fn release_train_prep_script_enforces_version_and_package_scope() {
    assert!(
        RELEASE_TRAIN_PREP_SCRIPT.contains("VERSION_RE = re.compile"),
        "prep script must validate MAJOR.MINOR.PATCH versions"
    );
    assert!(
        RELEASE_TRAIN_PREP_SCRIPT.contains("RELEASE_PACKAGE_RE = re.compile"),
        "prep script must derive release package scope from package names"
    );
    assert!(
        RELEASE_TRAIN_PREP_SCRIPT.contains("not VERSION_RE.fullmatch(package.version)"),
        "prep script must reject stale prerelease package versions"
    );
    assert!(
        RELEASE_TRAIN_PREP_SCRIPT.contains("cargo(\"test\", \"--test\", \"release_contract\", \"--test\", \"manual_release_matching\")"),
        "prep script must run focused release contract validation"
    );
    assert!(
        RELEASE_TRAIN_PREP_SCRIPT.contains("validate_lockfile_versions(version)"),
        "prep script must verify Cargo.lock release package versions"
    );
}

#[test]
fn release_workflows_use_shared_setup_build_and_signing_helpers() {
    for (name, workflow) in [
        ("nightly workflow", NIGHTLY_WORKFLOW),
        ("RC workflow", RC_WORKFLOW),
        ("stable workflow", STABLE_WORKFLOW),
    ] {
        assert!(
            workflow.contains("scripts/internal/release/checkout_radiant_submodule.sh"),
            "{name} must use the shared Radiant checkout helper"
        );
        assert!(
            workflow.contains("scripts/internal/release/emit_rust_toolchain_channel.py"),
            "{name} must use the shared Rust toolchain channel helper"
        );
        assert!(
            workflow.contains("scripts/internal/release/setup_windows_asio_sdk.ps1"),
            "{name} must use the shared Windows ASIO setup helper"
        );
        assert!(
            workflow.contains("scripts/internal/release/build_release_artifact.sh"),
            "{name} must use the shared release artifact build helper"
        );
        assert!(
            workflow.contains("scripts/internal/release/assemble_release_files.sh"),
            "{name} must use the shared release assembly helper"
        );
        assert!(
            workflow.contains("scripts/internal/release/sign_release_checksums.sh"),
            "{name} must use the shared checksum signing helper"
        );
    }
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW
            .contains("scripts/internal/release/checkout_radiant_submodule.sh"),
        "release train prep must use the shared Radiant checkout helper"
    );
    assert!(
        RELEASE_TRAIN_PREP_WORKFLOW
            .contains("scripts/internal/release/emit_rust_toolchain_channel.py"),
        "release train prep must use the shared Rust toolchain channel helper"
    );
    assert!(
        NIGHTLY_WORKFLOW.contains("scripts/internal/release/prune_github_release_assets.sh"),
        "nightly workflow must use the shared rolling-release asset pruning helper"
    );
}

#[test]
fn shared_release_helpers_keep_policy_visible_and_strict() {
    assert!(
        CHECKOUT_RADIANT_SCRIPT.contains("Missing RADIANT_SUBMODULE_DEPLOY_KEY"),
        "Radiant checkout helper must fail clearly when the deploy key is missing"
    );
    assert!(
        BUILD_RELEASE_ARTIFACT_SCRIPT.contains("--channel <nightly|rc|stable>"),
        "artifact build helper must keep release channel explicit"
    );
    assert!(
        BUILD_RELEASE_ARTIFACT_SCRIPT.contains("build_release_zip.sh"),
        "artifact build helper must delegate to the canonical zip packager"
    );
    assert!(
        ASSEMBLE_RELEASE_FILES_SCRIPT.contains("No checksums entry files found"),
        "release assembly helper must fail when checksum entries are missing"
    );
    assert!(
        SIGN_RELEASE_CHECKSUMS_SCRIPT.contains("Missing CHECKSUMS_SIGNING_KEY"),
        "checksum signing helper must fail clearly when the signing key is missing"
    );
    assert!(
        SIGN_RELEASE_CHECKSUMS_SCRIPT.contains("--verify-public-key"),
        "checksum signing helper must preserve public-key verification support"
    );
    assert!(
        VALIDATE_PROMOTED_RC_SCRIPT.contains("gh\","),
        "promoted RC validator must use GitHub CLI for live release validation"
    );
    assert!(
        VALIDATE_PROMOTED_RC_SCRIPT.contains("release_contract.toml"),
        "promoted RC validator must derive expected assets from the release contract"
    );
    assert!(
        VALIDATE_PROMOTED_RC_SCRIPT.contains("RC release is missing required assets"),
        "promoted RC validator must fail clearly when release assets are incomplete"
    );
    assert!(
        VALIDATE_PROMOTED_RC_SCRIPT.contains("Checksum mismatch"),
        "promoted RC validator must compare checksum entries to downloaded zip hashes"
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

fn apply_asset_template(
    template: &str,
    app_name: &str,
    target: &ReleaseTarget,
    version: &str,
    rc_number: &str,
) -> String {
    template
        .replace("{APP_NAME}", app_name)
        .replace("{version}", version)
        .replace("{rc_number}", rc_number)
        .replace("{platform}", &target.platform)
        .replace("{arch}", &target.arch)
}

fn workflow_release_targets(workflow: &str) -> BTreeSet<ReleaseTarget> {
    let mut targets = BTreeSet::new();
    let mut entry = BTreeMap::new();
    let mut in_matrix_include = false;

    for line in workflow.lines() {
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
