use std::fs;

use tempfile::tempdir;

use crate::{
    app_dirs::{APP_DIR_NAME, AppRootGuard, PersistenceProfileGuard},
    sample_sources::config::{AppConfig, AppSettingsCore},
};

use super::{
    FixtureMutation, FixtureName, FixtureProfile, FixtureProvisionRequest, apply_mutation,
    manifest_io::sha256,
    provision,
    provision::{manifest_path, validate},
};

fn request(base: &std::path::Path, fixture: FixtureName) -> FixtureProvisionRequest {
    FixtureProvisionRequest {
        config_base: base.to_path_buf(),
        fixture,
        profile: FixtureProfile::AutomatedTests,
        reset: true,
    }
}

#[test]
fn small_multi_source_has_stable_topology_counts_and_identities() {
    let base = tempdir().expect("fixture base");
    let manifest = provision(&request(base.path(), FixtureName::SmallMultiSource))
        .expect("provision small fixture");

    assert_eq!(manifest.sources.len(), 2);
    assert_eq!(manifest.expected_supported_file_count, 9);
    assert_eq!(manifest.expected_unsupported_file_count, 1);
    assert_eq!(manifest.expected_readiness_target_count, 29);
    assert_eq!(
        manifest
            .sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["fixture-small-alpha-v1", "fixture-small-beta-v1"]
    );
    assert!(manifest.sources[0].root.join("empty").is_dir());
    assert!(manifest.files.iter().any(|file| {
        file.relative_path == "drums/snare-stereo-48000.wav"
            && file.channels == Some(2)
            && file.sample_rate == Some(48_000)
            && file.frames == Some(18_000)
    }));
}

#[test]
fn repeated_reset_reproduces_identical_manifest_and_input_hashes() {
    let base = tempdir().expect("fixture base");
    let fixture_request = request(base.path(), FixtureName::SmallMultiSource);
    let first = provision(&fixture_request).expect("first provision");
    let first_bytes = fs::read(manifest_path(base.path(), FixtureName::SmallMultiSource))
        .expect("first manifest");

    apply_mutation(
        base.path().to_path_buf(),
        FixtureName::SmallMultiSource,
        FixtureProfile::AutomatedTests,
        FixtureMutation::SameSizeChange,
    )
    .expect("mutate fixture");
    let second = provision(&fixture_request).expect("second provision");
    let second_bytes = fs::read(manifest_path(base.path(), FixtureName::SmallMultiSource))
        .expect("second manifest");

    assert_eq!(first, second);
    assert_eq!(first_bytes, second_bytes);
    assert_eq!(
        first.files[0].sha256,
        "0b449469299da4d97969ca2c97fc219962d9e592d716b4e51569ccd4c52f7987"
    );
}

#[test]
fn validation_rejects_tampered_fixture_input() {
    let base = tempdir().expect("fixture base");
    let manifest =
        provision(&request(base.path(), FixtureName::SmallMultiSource)).expect("provision fixture");
    let source = &manifest.sources[0];
    let file = manifest
        .files
        .iter()
        .find(|file| file.source_id == source.source_id)
        .expect("source file");
    fs::write(source.root.join(&file.relative_path), b"tampered").expect("tamper fixture");
    let path = manifest_path(base.path(), FixtureName::SmallMultiSource);
    let mut manifest_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("manifest bytes")).expect("manifest JSON");
    let manifest_file = manifest_json["files"]
        .as_array_mut()
        .expect("manifest files")
        .iter_mut()
        .find(|entry| {
            entry["source_id"].as_str() == Some(file.source_id.as_str())
                && entry["relative_path"].as_str() == Some(file.relative_path.as_str())
        })
        .expect("matching manifest file");
    manifest_file["sha256"] = serde_json::Value::String(
        sha256(&source.root.join(&file.relative_path)).expect("tampered hash"),
    );
    fs::write(
        &path,
        serde_json::to_vec_pretty(&manifest_json).expect("tampered manifest JSON"),
    )
    .expect("write matching tampered manifest");

    let error = validate(
        base.path(),
        FixtureName::SmallMultiSource,
        FixtureProfile::AutomatedTests,
    )
    .expect_err("tampered fixture must fail");
    assert!(
        error.contains("versioned deterministic fixture contract"),
        "{error}"
    );
}

#[test]
fn validation_rejects_self_consistent_manifest_metadata_tampering() {
    let base = tempdir().expect("fixture base");
    provision(&request(base.path(), FixtureName::SmallMultiSource)).expect("provision fixture");
    let path = manifest_path(base.path(), FixtureName::SmallMultiSource);
    let mut manifest_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("manifest bytes")).expect("manifest JSON");
    manifest_json["expected_supported_file_count"] = serde_json::Value::from(10);
    manifest_json["sources"][0]["supported_file_count"] = serde_json::Value::from(5);
    fs::write(
        &path,
        serde_json::to_vec_pretty(&manifest_json).expect("tampered manifest JSON"),
    )
    .expect("write self-consistent tampered manifest");

    let error = validate(
        base.path(),
        FixtureName::SmallMultiSource,
        FixtureProfile::AutomatedTests,
    )
    .expect_err("tampered manifest metadata must fail");
    assert!(
        error.contains("versioned deterministic fixture contract"),
        "{error}"
    );
}

#[test]
fn empty_fixture_configures_no_sources() {
    let base = tempdir().expect("fixture base");
    let manifest = provision(&request(base.path(), FixtureName::Empty)).expect("empty fixture");

    assert!(manifest.sources.is_empty());
    assert!(manifest.files.is_empty());
    assert_eq!(manifest.expected_readiness_target_count, 0);
}

#[test]
fn provisioning_sandbox_leaves_live_profile_bytes_unchanged() {
    let base = tempdir().expect("fixture base");
    let live_paths = {
        let _live_guard = PersistenceProfileGuard::live();
        let _root_guard = AppRootGuard::set(base.path().join(APP_DIR_NAME))
            .expect("select isolated live sentinel root");
        crate::sample_sources::config::save(&AppConfig {
            sources: Vec::new(),
            core: AppSettingsCore {
                volume: 0.314,
                ..AppSettingsCore::default()
            },
        })
        .expect("seed live profile");
        let root = crate::app_dirs::app_root_dir().expect("live app root");
        (root.join("config.toml"), root.join("library.db"))
    };
    let before = (
        fs::read(&live_paths.0).expect("live config before"),
        fs::read(&live_paths.1).expect("live library before"),
    );

    provision(&FixtureProvisionRequest {
        config_base: base.path().to_path_buf(),
        fixture: FixtureName::SmallMultiSource,
        profile: FixtureProfile::Sandbox,
        reset: true,
    })
    .expect("sandbox fixture");

    assert_eq!(
        fs::read(&live_paths.0).expect("live config after"),
        before.0
    );
    assert_eq!(
        fs::read(&live_paths.1).expect("live library after"),
        before.1
    );
}

#[test]
fn mutation_scenarios_restore_to_the_same_baseline() {
    let base = tempdir().expect("fixture base");
    let fixture_request = request(base.path(), FixtureName::SmallMultiSource);
    let baseline = provision(&fixture_request).expect("baseline fixture");
    let source_beta = baseline.sources[1].root.clone();
    let original_change_size = fs::metadata(source_beta.join("mutable/change-me.wav"))
        .expect("change fixture metadata")
        .len();
    for mutation in [
        FixtureMutation::Create,
        FixtureMutation::SameSizeChange,
        FixtureMutation::Move,
        FixtureMutation::Delete,
    ] {
        apply_mutation(
            base.path().to_path_buf(),
            FixtureName::SmallMultiSource,
            FixtureProfile::AutomatedTests,
            mutation,
        )
        .expect("apply mutation");
        match mutation {
            FixtureMutation::Create => {
                assert!(source_beta.join("mutable/created.wav").is_file());
            }
            FixtureMutation::SameSizeChange => assert_eq!(
                fs::metadata(source_beta.join("mutable/change-me.wav"))
                    .expect("changed fixture metadata")
                    .len(),
                original_change_size
            ),
            FixtureMutation::Move => {
                assert!(!source_beta.join("mutable/move-me.wav").exists());
                assert!(source_beta.join("moved/move-me.wav").is_file());
            }
            FixtureMutation::Delete => {
                assert!(!source_beta.join("mutable/delete-me.wav").exists());
            }
            FixtureMutation::RootOffline | FixtureMutation::RootOnline | FixtureMutation::Reset => {
                unreachable!()
            }
        }
        apply_mutation(
            base.path().to_path_buf(),
            FixtureName::SmallMultiSource,
            FixtureProfile::AutomatedTests,
            FixtureMutation::Reset,
        )
        .expect("reset fixture");
        assert_eq!(
            validate(
                base.path(),
                FixtureName::SmallMultiSource,
                FixtureProfile::AutomatedTests,
            )
            .expect("validate reset"),
            baseline
        );
    }
}

#[test]
fn root_offline_and_online_scenario_restores_manifest_validity() {
    let base = tempdir().expect("fixture base");
    provision(&request(base.path(), FixtureName::SmallMultiSource)).expect("fixture");

    apply_mutation(
        base.path().to_path_buf(),
        FixtureName::SmallMultiSource,
        FixtureProfile::AutomatedTests,
        FixtureMutation::RootOffline,
    )
    .expect("offline");
    assert!(
        validate(
            base.path(),
            FixtureName::SmallMultiSource,
            FixtureProfile::AutomatedTests,
        )
        .is_err()
    );
    apply_mutation(
        base.path().to_path_buf(),
        FixtureName::SmallMultiSource,
        FixtureProfile::AutomatedTests,
        FixtureMutation::RootOnline,
    )
    .expect("online");
}

#[cfg(unix)]
#[test]
fn reset_refuses_symlinked_fixture_ancestors() {
    use std::os::unix::fs::symlink;

    let base = tempdir().expect("fixture base");
    let outside = tempdir().expect("outside root");
    let sentinel = outside.path().join("sentinel.txt");
    fs::write(&sentinel, b"must remain").expect("outside sentinel");
    symlink(outside.path(), base.path().join(APP_DIR_NAME)).expect("symlink fixture ancestor");

    let error = provision(&request(base.path(), FixtureName::SmallMultiSource))
        .expect_err("symlinked fixture ancestor must be rejected");

    assert!(error.contains("symbolic link"), "{error}");
    assert_eq!(
        fs::read(&sentinel).expect("outside sentinel remains"),
        b"must remain"
    );
}

#[cfg(unix)]
#[test]
fn mutations_refuse_a_source_root_swapped_for_an_external_symlink() {
    use std::os::unix::fs::symlink;

    let base = tempdir().expect("fixture base");
    let outside = tempdir().expect("outside root");
    let manifest =
        provision(&request(base.path(), FixtureName::SmallMultiSource)).expect("fixture");
    let source_beta = manifest.sources[1].root.clone();
    let external_source = outside.path().join("source-beta");
    fs::rename(&source_beta, &external_source).expect("move source outside fixture");
    symlink(&external_source, &source_beta).expect("swap source root for symlink");
    let protected = external_source.join("mutable/change-me.wav");
    let protected_before = fs::read(&protected).expect("external protected bytes");

    for mutation in [
        FixtureMutation::Create,
        FixtureMutation::SameSizeChange,
        FixtureMutation::Move,
        FixtureMutation::Delete,
        FixtureMutation::RootOffline,
    ] {
        let error = apply_mutation(
            base.path().to_path_buf(),
            FixtureName::SmallMultiSource,
            FixtureProfile::AutomatedTests,
            mutation,
        )
        .expect_err("symlinked source mutation must fail");
        assert!(error.contains("symbolic link"), "{mutation:?}: {error}");
    }
    assert_eq!(
        fs::read(&protected).expect("external protected bytes after mutations"),
        protected_before
    );
    assert!(!external_source.join("mutable/created.wav").exists());
    assert!(external_source.join("mutable/delete-me.wav").is_file());
    assert!(external_source.join("mutable/move-me.wav").is_file());
}
