//! Runtime access to the active release target matrix.
//!
//! `release_contract.toml` is the release-pipeline source of truth. The updater
//! uses the same target matrix here so asset lookup cannot infer support for
//! unpublished platform/architecture pairs.

use std::sync::OnceLock;

use serde::Deserialize;

use super::UpdateError;

const RELEASE_CONTRACT: &str = include_str!("../../release_contract.toml");

static ACTIVE_TARGETS: OnceLock<Result<Vec<ReleaseTarget>, String>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct ReleaseContract {
    targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleaseTarget {
    target: String,
    platform: String,
    arch: String,
}

/// Return whether `platform`/`arch` is an active published release target.
pub(super) fn supports_platform_arch(platform: &str, arch: &str) -> Result<bool, UpdateError> {
    Ok(active_targets()?
        .iter()
        .any(|target| target.platform == platform && target.arch == arch))
}

/// Return the active target triple for a published `platform`/`arch` pair.
pub fn supported_release_target_for_platform_arch(
    platform: &str,
    arch: &str,
) -> Option<&'static str> {
    active_targets().ok()?.iter().find_map(|target| {
        (target.platform == platform && target.arch == arch).then_some(target.target.as_str())
    })
}

fn active_targets() -> Result<&'static [ReleaseTarget], UpdateError> {
    match ACTIVE_TARGETS.get_or_init(load_active_targets) {
        Ok(targets) => Ok(targets.as_slice()),
        Err(message) => Err(UpdateError::Invalid(format!(
            "Invalid release contract: {message}"
        ))),
    }
}

fn load_active_targets() -> Result<Vec<ReleaseTarget>, String> {
    let contract: ReleaseContract = toml::from_str(RELEASE_CONTRACT)
        .map_err(|err| format!("release_contract.toml does not parse: {err}"))?;
    let mut targets = Vec::with_capacity(contract.targets.len());
    for target in contract.targets {
        targets.push(release_target_from_triple(&target)?);
    }
    Ok(targets)
}

fn release_target_from_triple(target: &str) -> Result<ReleaseTarget, String> {
    let Some((arch, _)) = target.split_once('-') else {
        return Err(format!(
            "target '{target}' is missing an architecture prefix"
        ));
    };
    let platform = if target.contains("-pc-windows-") {
        "windows"
    } else if target.ends_with("-apple-darwin") {
        "macos"
    } else {
        return Err(format!(
            "target '{target}' is not a supported release target"
        ));
    };
    Ok(ReleaseTarget {
        target: target.to_string(),
        platform: platform.to_string(),
        arch: arch.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_contract_supports_current_published_targets() {
        assert!(supports_platform_arch("windows", "x86_64").unwrap());
        assert!(supports_platform_arch("macos", "x86_64").unwrap());
        assert!(supports_platform_arch("macos", "aarch64").unwrap());
    }

    #[test]
    fn release_contract_rejects_unpublished_targets() {
        assert!(!supports_platform_arch("linux", "x86_64").unwrap());
        assert!(!supports_platform_arch("windows", "aarch64").unwrap());
    }

    #[test]
    fn release_contract_resolves_target_triples_for_supported_pairs() {
        assert_eq!(
            supported_release_target_for_platform_arch("windows", "x86_64"),
            Some("x86_64-pc-windows-msvc")
        );
        assert_eq!(
            supported_release_target_for_platform_arch("linux", "x86_64"),
            None
        );
    }
}
